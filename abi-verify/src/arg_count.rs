//! Heuristic recovery of the number of parameters a raw x86-64 function
//! reads, per the Microsoft x64 calling convention.

use iced_x86::{
    Decoder, DecoderOptions, FlowControl, Instruction, InstructionInfo, InstructionInfoFactory, Mnemonic, OpAccess,
    OpKind, Register,
};
use std::collections::{HashMap, HashSet};

/// Maximum number of instructions to decode within a single basic block
/// before treating it as a dead end. Guards against a block that never hits
/// a recognized terminator (e.g. a gap in flow-control detection).
const BLOCK_INSTRUCTION_BUDGET: usize = 200;

/// Maximum total instructions to decode across every explored block. Real
/// functions read their incoming args long before this is exhausted; this
/// just bounds the work for pathological/garbage input or a CFG with an
/// unreasonable number of blocks.
const GLOBAL_INSTRUCTION_BUDGET: usize = 3000;

/// Result of [`recover_arg_count`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredArgCount {
    /// How many of RCX, RDX, R8, R9 were read before being clobbered (0..=4).
    pub register_args: u8,
    /// How many additional stack-passed args (5th+) were read before being
    /// clobbered.
    pub stack_args: u8,
    /// Widest byte width (1/2/4/8) any explored path read each of RCX, RDX,
    /// R8, R9 at while still a live incoming argument; `None` if never read
    /// live.
    ///
    /// **Unreliable for type inference — do not enforce these.** MSVC
    /// prologues spill all four register args to their shadow-space home
    /// slots with full 64-bit stores regardless of the arg's logical width,
    /// so a bool or `i32` arg very often shows width 8. Debugging only.
    pub register_arg_widths: [Option<u8>; 4],
    /// Widest byte width observed reading each stack-passed arg slot, indexed
    /// by stack position (0 = entry-relative `[rsp+0x28]`, 1 = `+0x30`, ...);
    /// `None` for a slot below the high-water mark that was never read.
    /// Unlike register widths these are meaningful — stack slots are read at
    /// their real width — though still a heuristic.
    pub stack_arg_widths: Vec<Option<u8>>,
    /// `true` if any of XMM0..XMM3 was read live on some path — i.e. the
    /// function takes a float/vector argument. A float arg consumes an
    /// argument *slot index* shared with the integer registers, so its
    /// presence shifts the position-to-stack-slot mapping and invalidates the
    /// per-slot width analysis — don't enforce widths when this is set. No
    /// current FFI target takes floats; this guards against miscounting one.
    pub has_float_args: bool,
}

impl RecoveredArgCount {
    /// Total number of parameter slots observed being read.
    #[must_use]
    pub const fn total(&self) -> u8 {
        self.register_args + self.stack_args
    }

    /// The `(register_args, stack_args)` pair, ignoring width detail — a
    /// convenience for count-focused assertions.
    #[cfg(test)]
    const fn counts(&self) -> (u8, u8) {
        (self.register_args, self.stack_args)
    }
}

/// Reads-before-write tracking for one of the four integer/pointer argument
/// registers (RCX, RDX, R8, R9), as observed along one explored path.
#[derive(Default, Clone, Copy)]
struct RegState {
    used: bool,
    clobbered: bool,
    /// Widest byte width this register was read at while still live
    /// (pre-clobber) on this path. 0 if never read live.
    width: u8,
}

impl RegState {
    fn observe(&mut self, access: OpAccess, reg_width: u8) {
        if !self.clobbered && is_read(access) {
            self.used = true;
            self.width = self.width.max(reg_width);
        }
        if is_write(access) {
            self.clobbered = true;
        }
    }
}

const fn is_read(access: OpAccess) -> bool {
    matches!(access, OpAccess::Read | OpAccess::CondRead | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

const fn is_write(access: OpAccess) -> bool {
    matches!(access, OpAccess::Write | OpAccess::CondWrite | OpAccess::ReadWrite | OpAccess::ReadCondWrite)
}

/// The path-local state needed to resume analysis at a given point: register
/// read/clobber tracking plus enough stack bookkeeping to map a `[rsp+disp]`
/// or `[rbp+disp]` read back to an entry-relative offset.
#[derive(Default, Clone, Copy)]
struct PathState {
    rsp_delta: i64,
    /// `Some(rbp - entry_rsp)` once a `lea rbp, [rsp+imm]` / `mov rbp, rsp`
    /// frame-pointer setup has been observed on this path. Unlike RSP, RBP is
    /// a fixed anchor once established, so this stays valid for the rest of
    /// the path.
    ///
    /// ponytail: doesn't detect RBP being repurposed for something else
    /// later in the function (real compiler-generated prologues don't do
    /// this) — revisit if a target function's frame pointer gets reused.
    rbp_frame_offset: Option<i64>,
    rcx: RegState,
    rdx: RegState,
    r8: RegState,
    r9: RegState,
    /// Read-before-write tracking for XMM0..XMM3 (the float/vector argument
    /// registers), used only to detect whether the function takes a float
    /// arg — see `RecoveredArgCount::has_float_args`.
    xmm: [RegState; 4],
}

/// One entry in the block-exploration worklist: where to resume decoding plus
/// the path-local state at that point. Copied into every block discovered via
/// a branch/fallthrough, so divergent paths never alias each other's state.
#[derive(Clone, Copy)]
struct BlockState {
    offset: usize,
    path: PathState,
}

/// Applies one decoded instruction's effect to the running path state:
/// register used/clobbered bookkeeping, stack-read bookkeeping (raising
/// `stack_high_water`), and RSP-delta/frame-pointer tracking.
fn observe_instruction(
    instruction: &Instruction,
    info: &InstructionInfo,
    path: &mut PathState,
    stack_high_water: &mut i64,
    stack_widths: &mut HashMap<i64, u8>,
) {
    for used_reg in info.used_registers() {
        let raw = used_reg.register();
        let reg_state = match raw.full_register() {
            Register::RCX => Some(&mut path.rcx),
            Register::RDX => Some(&mut path.rdx),
            Register::R8 => Some(&mut path.r8),
            Register::R9 => Some(&mut path.r9),
            _ if raw.is_vector_register() && raw.number() < 4 => Some(&mut path.xmm[raw.number()]),
            _ => None,
        };
        if let Some(reg_state) = reg_state {
            // The accessed sub-register's byte width (RCX=8, ECX=4, CX=2,
            // CL=1), read before `full_register()` collapses them all to RCX.
            let reg_width = u8::try_from(raw.size()).unwrap_or(u8::MAX);
            reg_state.observe(used_reg.access(), reg_width);
        }
    }

    for used_mem in info.used_memory() {
        if !is_read(used_mem.access()) {
            continue;
        }
        let entry_relative = if used_mem.base() == Register::RSP {
            Some(used_mem.displacement() as i64 - path.rsp_delta)
        } else if used_mem.base() == Register::RBP {
            path.rbp_frame_offset.map(|frame_offset| frame_offset + used_mem.displacement() as i64)
        } else {
            None
        };
        if let Some(entry_relative) = entry_relative
            && entry_relative >= 0x28
        {
            *stack_high_water = (*stack_high_water).max(entry_relative);
            let read_width = u8::try_from(used_mem.memory_size().size()).unwrap_or(u8::MAX);
            let slot = stack_widths.entry(entry_relative).or_insert(0);
            *slot = (*slot).max(read_width);
        }
    }

    // Recognize the common MSVC frame-pointer setup so later `[rbp+disp]`
    // reads can still be mapped back to entry-relative stack-arg offsets,
    // even though RBP stops moving in lockstep with RSP from this point on.
    match instruction.mnemonic() {
        Mnemonic::Lea if instruction.op0_register() == Register::RBP && instruction.memory_base() == Register::RSP => {
            path.rbp_frame_offset = Some(instruction.memory_displacement64() as i64 - path.rsp_delta);
        }
        Mnemonic::Mov if instruction.op0_register() == Register::RBP && instruction.op1_register() == Register::RSP => {
            path.rbp_frame_offset = Some(-path.rsp_delta);
        }
        Mnemonic::Sub if instruction.op0_register() == Register::RSP => {
            path.rsp_delta += instruction.immediate(1) as i64;
        }
        Mnemonic::Add if instruction.op0_register() == Register::RSP => {
            path.rsp_delta -= instruction.immediate(1) as i64;
        }
        Mnemonic::Push => path.rsp_delta += 8,
        Mnemonic::Pop => path.rsp_delta -= 8,
        _ => {}
    }
}

/// Resolves a near-branch instruction's target to a byte offset into `code`
/// (treating `code[0]` as virtual address 0), or `None` if the instruction
/// isn't a near branch (e.g. it's an indirect/far branch we can't statically
/// resolve) or the target falls outside `code`.
fn near_branch_offset(instruction: &Instruction, code_len: usize) -> Option<usize> {
    let is_near = matches!(instruction.op0_kind(), OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64);
    if !is_near {
        return None;
    }
    let target = instruction.near_branch_target() as usize;
    (target < code_len).then_some(target)
}

/// Recovers how many incoming parameters a function appears to read.
///
/// Takes raw machine code starting at the function's entry point and counts
/// parameter slots per the Microsoft x64 calling convention (RCX, RDX, R8,
/// R9, then stack from `[rsp+0x28]`).
///
/// A heuristic, not a decompiler-grade recovery pass: it explores basic
/// blocks from a bounded worklist, following both sides of every conditional
/// branch and every unconditional near jump. A slot counts as read if it was
/// read live on *any* explored path before being clobbered *on that path*, so
/// a conditional early-exit can't hide args read on the other branch.
/// Indirect and far branches aren't followed (their targets aren't statically
/// known); decoding continues past them.
///
/// Recognizes RSP-relative stack args and the common MSVC frame-pointer setup
/// (`lea rbp, [rsp+imm]` / `mov rbp, rsp`), but not more exotic addressing.
///
/// Register args conventionally have no gaps (a function reading R8 but never
/// RDX would be unusual), but this reports only what it observed — treating a
/// gap as suspicious is the caller's job.
#[must_use]
#[allow(
    clippy::similar_names,
    reason = "rcx/rdx/r8/r9 mirror the x64 register names being tracked; renaming them would obscure, not clarify"
)]
pub fn recover_arg_count(code: &[u8]) -> RecoveredArgCount {
    let mut info_factory = InstructionInfoFactory::new();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut worklist: Vec<BlockState> = vec![BlockState { offset: 0, path: PathState::default() }];

    // Ever-growing accumulator: once a bit is raised, it's never reset. This
    // is the union of "used on some path" across every explored path.
    let mut any_path_used = [false; 4]; // [rcx, rdx, r8, r9]
    let mut register_max_width = [0u8; 4]; // widest live read of each, across paths
    let mut any_xmm_used = [false; 4]; // XMM0..XMM3 read live on some path
    let mut stack_high_water: i64 = 0;
    // Widest read width per entry-relative stack offset, unioned across paths.
    let mut stack_widths: HashMap<i64, u8> = HashMap::new();
    let mut total_instructions = 0usize;

    while let Some(block) = worklist.pop() {
        if total_instructions >= GLOBAL_INSTRUCTION_BUDGET {
            break;
        }
        if block.offset >= code.len() || !visited.insert(block.offset) {
            continue;
        }

        let mut decoder = Decoder::new(64, &code[block.offset..], DecoderOptions::NONE);
        decoder.set_ip(block.offset as u64);
        let mut path = block.path;

        let mut block_instructions = 0usize;
        loop {
            if block_instructions >= BLOCK_INSTRUCTION_BUDGET
                || total_instructions >= GLOBAL_INSTRUCTION_BUDGET
                || !decoder.can_decode()
            {
                break;
            }
            let instruction = decoder.decode();
            block_instructions += 1;
            total_instructions += 1;

            let info = info_factory.info(&instruction);
            observe_instruction(&instruction, info, &mut path, &mut stack_high_water, &mut stack_widths);

            if matches!(instruction.mnemonic(), Mnemonic::Ret | Mnemonic::Retf) {
                break;
            }

            match instruction.flow_control() {
                FlowControl::UnconditionalBranch => {
                    if let Some(target) = near_branch_offset(&instruction, code.len()) {
                        worklist.push(BlockState { offset: target, path });
                    }
                    break;
                }
                FlowControl::ConditionalBranch => {
                    if let Some(target) = near_branch_offset(&instruction, code.len()) {
                        worklist.push(BlockState { offset: target, path });
                    }
                    let fallthrough = instruction.next_ip() as usize;
                    if fallthrough < code.len() {
                        worklist.push(BlockState { offset: fallthrough, path });
                    }
                    break;
                }
                _ => {}
            }
        }

        for (i, reg) in [path.rcx, path.rdx, path.r8, path.r9].into_iter().enumerate() {
            any_path_used[i] |= reg.used;
            register_max_width[i] = register_max_width[i].max(reg.width);
        }
        for (i, xmm) in path.xmm.into_iter().enumerate() {
            any_xmm_used[i] |= xmm.used;
        }
    }

    let register_args = any_path_used.into_iter().map(u8::from).sum();
    let stack_args = if stack_high_water >= 0x28 { ((stack_high_water - 0x20) / 8) as u8 } else { 0 };

    let register_arg_widths = std::array::from_fn(|i| any_path_used[i].then_some(register_max_width[i]));
    // Index j corresponds to entry-relative offset 0x28 + j*8; `None` for a
    // slot that's below the high-water mark yet was never actually read.
    let stack_arg_widths =
        (0..stack_args as usize).map(|j| stack_widths.get(&(0x28 + (j as i64) * 8)).copied()).collect();
    let has_float_args = any_xmm_used.into_iter().any(|used| used);

    RecoveredArgCount { register_args, stack_args, register_arg_widths, stack_arg_widths, has_float_args }
}

#[cfg(test)]
mod tests {
    use super::{recover_arg_count, RecoveredArgCount};
    use iced_x86::{Code, Encoder, Instruction, MemoryOperand, Register};

    /// Encodes instructions into raw bytes as if they were a function at its
    /// entry point. Each is encoded at its real running byte offset (not
    /// always IP 0), so near-branch displacements match the offset convention
    /// `recover_arg_count` uses (`code[0]` == virtual address 0).
    fn assemble(instructions: &[Instruction]) -> Vec<u8> {
        let mut encoder = Encoder::new(64);
        let mut rip = 0u64;
        for instruction in instructions {
            let len = encoder.encode(instruction, rip).expect("test instruction must encode");
            rip += len as u64;
        }
        encoder.take_buffer()
    }

    /// Byte offset each instruction would land at if assembled via
    /// [`assemble`], for computing real branch targets in a following pass.
    /// The `*_rel32_64` branch variants have a fixed encoded length
    /// regardless of target, so one probe pass yields final offsets.
    fn instruction_offsets(instructions: &[Instruction]) -> Vec<u64> {
        let mut encoder = Encoder::new(64);
        let mut rip = 0u64;
        let mut offsets = Vec::with_capacity(instructions.len());
        for instruction in instructions {
            offsets.push(rip);
            let len = encoder.encode(instruction, rip).expect("test instruction must encode (offset probe)");
            rip += len as u64;
        }
        offsets
    }

    fn ret() -> Instruction {
        Instruction::with(Code::Retnq)
    }

    /// `test eax, eax` — sets flags without touching any argument register.
    fn test_eax_eax() -> Instruction {
        Instruction::with2(Code::Test_rm32_r32, Register::EAX, Register::EAX)
            .expect("test eax, eax must encode")
    }

    fn jmp(target: u64) -> Instruction {
        Instruction::with_branch(Code::Jmp_rel32_64, target).expect("jmp must encode")
    }

    fn je(target: u64) -> Instruction {
        Instruction::with_branch(Code::Je_rel32_64, target).expect("je must encode")
    }

    fn jne(target: u64) -> Instruction {
        Instruction::with_branch(Code::Jne_rel32_64, target).expect("jne must encode")
    }

    /// `mov [rsp+disp], reg64` — spills a 64-bit register to the stack.
    fn spill64(disp: i64, reg: Register) -> Instruction {
        Instruction::with2(Code::Mov_rm64_r64, MemoryOperand::with_base_displ(Register::RSP, disp), reg)
            .expect("mov [rsp+disp], reg64 must encode")
    }

    /// `mov [rsp+disp], reg32` — spills a 32-bit sub-register to the stack.
    fn spill32(disp: i64, reg: Register) -> Instruction {
        Instruction::with2(Code::Mov_rm32_r32, MemoryOperand::with_base_displ(Register::RSP, disp), reg)
            .expect("mov [rsp+disp], reg32 must encode")
    }

    /// `mov rax, [rsp+disp]` — reads a stack slot.
    fn load_stack_arg(disp: i64) -> Instruction {
        Instruction::with2(Code::Mov_r64_rm64, Register::RAX, MemoryOperand::with_base_displ(Register::RSP, disp))
            .expect("mov rax, [rsp+disp] must encode")
    }

    fn sub_rsp(imm: u32) -> Instruction {
        Instruction::with2(Code::Sub_rm64_imm32, Register::RSP, imm).expect("sub rsp, imm must encode")
    }

    fn xor_ecx_ecx() -> Instruction {
        Instruction::with2(Code::Xor_r32_rm32, Register::ECX, Register::ECX).expect("xor ecx, ecx must encode")
    }

    #[test]
    fn only_rcx_read() {
        let code = assemble(&[spill64(-8, Register::RCX), ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (1, 0));
    }

    #[test]
    fn all_four_register_args_read() {
        let code = assemble(&[
            spill64(-8, Register::RCX),
            spill64(-16, Register::RDX),
            spill64(-24, Register::R8),
            spill64(-32, Register::R9),
            ret(),
        ]);
        assert_eq!(recover_arg_count(&code).counts(), (4, 0));
    }

    #[test]
    fn register_args_plus_one_stack_arg_no_sub_rsp() {
        let code = assemble(&[
            spill64(-8, Register::RCX),
            spill64(-16, Register::RDX),
            spill64(-24, Register::R8),
            spill64(-32, Register::R9),
            load_stack_arg(0x28),
            ret(),
        ]);
        assert_eq!(recover_arg_count(&code).counts(), (4, 1));
    }

    #[test]
    fn stack_arg_after_sub_rsp_normalizes_via_rsp_delta() {
        // After `sub rsp, 0x20`, the location that was [entry_rsp+0x28] is
        // now reached via [rsp+0x48]. This test nails the sign convention:
        // entry_relative = raw_displacement - rsp_delta.
        let code = assemble(&[sub_rsp(0x20), load_stack_arg(0x48), ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (0, 1));
    }

    #[test]
    fn rcx_clobbered_before_read_does_not_count() {
        let code = assemble(&[xor_ecx_ecx(), ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (0, 0));
    }

    #[test]
    fn sub_register_write_normalizes_to_full_register() {
        // `mov [rsp+4], ecx` must still count as RCX being used.
        let code = assemble(&[spill32(4, Register::ECX), ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (1, 0));
    }

    #[test]
    fn trivial_ret_only() {
        let code = assemble(&[ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (0, 0));
    }

    #[test]
    fn total_sums_register_and_stack_args() {
        let recovered = RecoveredArgCount {
            register_args: 4,
            stack_args: 2,
            register_arg_widths: [Some(8); 4],
            stack_arg_widths: vec![Some(8), Some(4)],
            has_float_args: false,
        };
        assert_eq!(recovered.total(), 6);
    }

    #[test]
    fn register_read_width_is_recovered() {
        // `mov [rsp-8], rcx` reads RCX as 8 bytes; `mov [rsp-16], edx` reads
        // RDX as 4 bytes. R8/R9 never read -> None.
        let code = assemble(&[spill64(-8, Register::RCX), spill32(-16, Register::EDX), ret()]);
        let recovered = recover_arg_count(&code);
        assert_eq!(recovered.register_arg_widths, [Some(8), Some(4), None, None]);
    }

    #[test]
    fn stack_read_width_distinguishes_8_from_4_bytes() {
        // Read [rsp+0x28] as 8 bytes (mov rax, ...) and [rsp+0x30] as 4 bytes
        // (mov eax, ...). The recovered stack widths must reflect that.
        let load32 = Instruction::with2(
            Code::Mov_r32_rm32,
            Register::EAX,
            MemoryOperand::with_base_displ(Register::RSP, 0x30),
        )
        .expect("mov eax, [rsp+0x30] must encode");
        let code = assemble(&[load_stack_arg(0x28), load32, ret()]);
        let recovered = recover_arg_count(&code);
        assert_eq!(recovered.stack_arg_widths, vec![Some(8), Some(4)]);
    }

    #[test]
    fn both_branch_paths_are_explored_and_merged() {
        // test eax, eax; je taken; <fallthrough: reads RCX; ret>
        // taken: reads RDX; ret
        //
        // A linear scan sees only the fallthrough path and would report 1
        // (RCX). Both sides must be explored, giving 2.
        let placeholder = [
            test_eax_eax(),
            je(0), // target fixed up below
            spill64(-8, Register::RCX),
            ret(),
            spill64(-16, Register::RDX), // taken-branch target
            ret(),
        ];
        let taken_target = instruction_offsets(&placeholder)[4];
        let code = assemble(&[
            test_eax_eax(),
            je(taken_target),
            spill64(-8, Register::RCX),
            ret(),
            spill64(-16, Register::RDX),
            ret(),
        ]);
        assert_eq!(recover_arg_count(&code).counts(), (2, 0));
    }

    #[test]
    fn unconditional_jmp_over_dead_bytes_is_followed() {
        // jmp target; <dead: ret> ; target: reads R8; ret
        //
        // The dead `ret` sits right after the jmp, so if the jmp isn't
        // followed it gets hit immediately and R8 is never counted.
        let placeholder = [jmp(0), ret(), spill64(-8, Register::R8), ret()];
        let target = instruction_offsets(&placeholder)[2];
        let code = assemble(&[jmp(target), ret(), spill64(-8, Register::R8), ret()]);
        assert_eq!(recover_arg_count(&code).counts(), (1, 0));
    }

    #[test]
    fn backward_loop_terminates_and_merges_both_reads() {
        // reads RCX
        // loop_top: test eax, eax; jne loop_top (backward)
        // reads RDX; ret
        //
        // Without the `visited` set stopping the loop target from being
        // reprocessed, this hangs or blows the budget without counting RDX.
        let placeholder = [spill64(-8, Register::RCX), test_eax_eax(), jne(0), spill64(-16, Register::RDX), ret()];
        let loop_top = instruction_offsets(&placeholder)[1];
        let code = assemble(&[
            spill64(-8, Register::RCX),
            test_eax_eax(),
            jne(loop_top),
            spill64(-16, Register::RDX),
            ret(),
        ]);
        assert_eq!(recover_arg_count(&code).counts(), (2, 0));
    }

    /// Ground-truth regression test: `CharacterDataStack::Push` in the real
    /// game binary is independently known (Binary Ninja) to read 18 params.
    /// Its early conditional branches sit before the code reading most of
    /// those args — the exact shape that defeated the old linear scan, which
    /// recovered only 2.
    #[test]
    #[ignore = "requires the actual League of Legends installation; run explicitly with `cargo test -p abi-verify -- --ignored`"]
    fn recovers_the_real_character_data_stack_push_signature() {
        let exe_path = r"C:\Riot Games\League of Legends\Game\League of Legends.exe";
        if std::fs::metadata(exe_path).is_err() {
            return;
        }
        let file_data = std::fs::read(exe_path).expect("game exe must be readable");

        let e_lfanew = u32::from_le_bytes(file_data[0x3C..0x40].try_into().unwrap()) as usize;
        let coff_header = e_lfanew + 4;
        let number_of_sections = u16::from_le_bytes(file_data[coff_header + 2..coff_header + 4].try_into().unwrap());
        let size_of_optional_header =
            u16::from_le_bytes(file_data[coff_header + 16..coff_header + 18].try_into().unwrap());
        let section_table = coff_header + 20 + size_of_optional_header as usize;

        let mut text_section = None;
        for i in 0..number_of_sections as usize {
            let section_offset = section_table + i * 40;
            let name = &file_data[section_offset..section_offset + 8];
            if name == b".text\0\0\0" {
                let virtual_address =
                    u32::from_le_bytes(file_data[section_offset + 12..section_offset + 16].try_into().unwrap());
                let size_of_raw_data =
                    u32::from_le_bytes(file_data[section_offset + 16..section_offset + 20].try_into().unwrap());
                let pointer_to_raw_data =
                    u32::from_le_bytes(file_data[section_offset + 20..section_offset + 24].try_into().unwrap());
                text_section = Some((virtual_address, pointer_to_raw_data, size_of_raw_data));
                break;
            }
        }
        let (text_rva, pointer_to_raw_data, size_of_raw_data) =
            text_section.expect(".text section must be present in a real PE image");

        let text_start_in_file = pointer_to_raw_data as usize;
        let text_section_bytes = &file_data[text_start_in_file..text_start_in_file + size_of_raw_data as usize];

        // CharacterDataStack::Push, image base 0x140000000, entry point
        // 0x14022b750 => RVA 0x22b750. (Not 0x14022b774, sometimes quoted for
        // this function — that's 0x24 bytes into the prologue, the first line
        // Binary Ninja's HLIL renders, not the raw entry point.)
        let offset_within_text = 0x0022_b750_usize - text_rva as usize;
        let push_bytes = &text_section_bytes[offset_within_text..];

        let recovered = recover_arg_count(push_bytes);
        assert_eq!(
            recovered.total(),
            18,
            "expected 18 total params for CharacterDataStack::Push, got {recovered:?}"
        );

        eprintln!("register_arg_widths = {:?}", recovered.register_arg_widths);
        eprintln!("stack_arg_widths    = {:?}", recovered.stack_arg_widths);

        // Push's 18 declared params (see `PushFn` in
        // WildSkin-rs/src/sdk/character_data.rs): 4 register args, then 14
        // stack args — flag×6 (bool), gear(i8), str2(ptr), i32, str4(ptr),
        // i32, flag(bool), i32, str1(ptr). The three stack-passed POINTER
        // args (indices 7, 9, 13) are the reliable signal: they must be read
        // at 8 bytes. Register widths aren't asserted — shadow-space
        // homogenization makes them meaningless (see the field doc).
        assert_eq!(recovered.stack_arg_widths.len(), 14, "expected 14 stack slots");
        for &pointer_slot in &[7usize, 9, 13] {
            assert_eq!(
                recovered.stack_arg_widths[pointer_slot],
                Some(8),
                "stack slot {pointer_slot} is a declared pointer arg and must be read at 8 bytes"
            );
        }
    }
}
