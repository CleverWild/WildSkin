//! Hooks for DirectX 11.

use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use imgui::Context;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use tracing::{error, trace};
use windows::core::{Error, Interface, Result, BOOL, HRESULT};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGISwapChain, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect};

use super::DummyHwnd;
use crate::mh::MhHook;
use crate::renderer::{D3D11RenderEngine, Pipeline};
use crate::{perform_eject, Hooks, ImguiRenderLoop, EJECT_REQUESTED, HOOK_EJECTION_BARRIER};

type DXGISwapChainPresentType =
    unsafe extern "system" fn(this: IDXGISwapChain, sync_interval: u32, flags: u32) -> HRESULT;

type DXGISwapChainResizeBuffersType = unsafe extern "system" fn(
    this: IDXGISwapChain,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT;

struct Trampolines {
    dxgi_swap_chain_present: DXGISwapChainPresentType,
    dxgi_swap_chain_resize_buffers: DXGISwapChainResizeBuffersType,
}

static mut TRAMPOLINES: OnceLock<Trampolines> = OnceLock::new();
static mut PIPELINE: OnceCell<Mutex<Pipeline<D3D11RenderEngine>>> = OnceCell::new();
static mut RENDER_LOOP: OnceCell<Box<dyn ImguiRenderLoop + Send + Sync>> = OnceCell::new();

unsafe fn init_pipeline(swap_chain: &IDXGISwapChain) -> Result<Mutex<Pipeline<D3D11RenderEngine>>> {
    let desc = swap_chain.GetDesc()?;
    let hwnd = desc.OutputWindow;

    let mut ctx = Context::create();
    let engine = D3D11RenderEngine::new(&swap_chain.GetDevice()?, &mut ctx)?;

    let Some(render_loop) = RENDER_LOOP.take() else {
        error!("Render loop not yet initialized");
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    let pipeline = Pipeline::new(hwnd, ctx, engine, render_loop).map_err(|(e, render_loop)| {
        RENDER_LOOP.get_or_init(move || render_loop);
        e
    })?;

    Ok(Mutex::new(pipeline))
}

fn render(swap_chain: &IDXGISwapChain) -> Result<()> {
    unsafe {
        let pipeline = PIPELINE.get_or_try_init(|| init_pipeline(swap_chain))?;

        let Some(mut pipeline) = pipeline.try_lock() else {
            error!("Could not lock pipeline");
            return Err(Error::from_hresult(HRESULT(-1)));
        };

        if let Ok(desc) = swap_chain.GetDesc() {
            pipeline
                .update_display_size_from_swap_chain(desc.BufferDesc.Width, desc.BufferDesc.Height);
        }

        pipeline.prepare_render()?;

        let target: ID3D11Texture2D = swap_chain.GetBuffer(0)?;

        pipeline.render(target)?;
    }
    Ok(())
}

unsafe extern "system" fn dxgi_swap_chain_present_impl(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    let _hook_ejection_guard = HOOK_EJECTION_BARRIER.acquire_ejection_guard();

    let Trampolines { dxgi_swap_chain_present, .. } =
        TRAMPOLINES.get().expect("DirectX 11 trampolines uninitialized");

    if let Err(e) = render(&swap_chain) {
        error!("Render error: {e:?}");
    }

    trace!("Call IDXGISwapChain::Present trampoline");
    let result = dxgi_swap_chain_present(swap_chain, sync_interval, flags);
    if EJECT_REQUESTED.load(Ordering::SeqCst) {
        perform_eject();
    }
    result
}

unsafe extern "system" fn dxgi_swap_chain_resize_buffers_impl(
    swap_chain: IDXGISwapChain,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT {
    let _hook_ejection_guard = HOOK_EJECTION_BARRIER.acquire_ejection_guard();

    let Trampolines { dxgi_swap_chain_resize_buffers, .. } =
        TRAMPOLINES.get().expect("DirectX 11 trampolines uninitialized");

    trace!("Call IDXGISwapChain::ResizeBuffers trampoline");
    let result =
        dxgi_swap_chain_resize_buffers(swap_chain, buffer_count, width, height, new_format, flags);

    if result.is_ok() {
        if let Some(pipeline) = PIPELINE.get() {
            if let Some(mut pipeline_guard) = pipeline.try_lock() {
                pipeline_guard.update_display_size_from_swap_chain(width, height);
            }
        }
    }

    result
}

/// Returns the LIVE addresses of the `Present`/`ResizeBuffers` slots inside
/// the shared `IDXGISwapChain` vtable (not just their current values).
/// `Interface::vtable()` dereferences the swapchain's own vtable pointer, so
/// the returned reference points at the same array every real swapchain of
/// this driver/type shares — patching a slot here hooks all of them, exactly
/// like patching the function body at a shared code address would, just one
/// pointer-sized write instead of a code patch.
fn get_target_vtable_slots() -> (*mut usize, *mut usize) {
    let mut p_device: Option<ID3D11Device> = None;
    let mut p_context: Option<ID3D11DeviceContext> = None;
    let mut p_swap_chain: Option<IDXGISwapChain> = None;

    let dummy_hwnd = DummyHwnd::new();
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE(std::ptr::null_mut()),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC {
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                    Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                    ..Default::default()
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 1,
                OutputWindow: dummy_hwnd.hwnd(),
                Windowed: BOOL(1),
                SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, ..Default::default() },
                ..Default::default()
            }),
            Some(&mut p_swap_chain),
            Some(&mut p_device),
            None,
            Some(&mut p_context),
        )
        .expect("D3D11CreateDeviceAndSwapChain failed");
    }

    let swap_chain = p_swap_chain.unwrap();

    // SAFETY: `vtable()` derefs the live swapchain's vtable pointer; taking
    // the address of a field on that place points into the real, shared
    // vtable array rather than a local copy.
    let present_slot = (&raw const swap_chain.vtable().Present).cast_mut().cast::<usize>();
    // SAFETY: as above.
    let resize_buffers_slot =
        (&raw const swap_chain.vtable().ResizeBuffers).cast_mut().cast::<usize>();

    (present_slot, resize_buffers_slot)
}

/// Overwrites a single pointer-sized vtable slot with `new_fn`, returning the
/// value it held before. A vtable slot is one aligned, pointer-sized write —
/// atomic on x86-64 (`AtomicUsize::from_ptr` makes that explicit rather than
/// relying on an implicit hardware guarantee) — so unlike inline code
/// patching, no other thread needs to be paused for this to stay safe: a
/// concurrent caller either reads the old pointer or the new one, never a
/// torn value.
///
/// # Safety
///
/// `slot` must point at a live, pointer-sized, writable-once-unprotected
/// vtable entry.
unsafe fn patch_vtable_slot(slot: *mut usize, new_fn: usize) -> usize {
    let mut old_protect = PAGE_PROTECTION_FLAGS(0);
    // SAFETY: `slot` is a valid pointer-sized location per the fn's own
    // safety contract; `size_of::<usize>()` matches exactly what is read/
    // written below.
    unsafe { VirtualProtect(slot.cast(), size_of::<usize>(), PAGE_READWRITE, &mut old_protect) }
        .expect("VirtualProtect (make vtable slot writable) failed");

    // SAFETY: `slot` was just made writable above.
    let original = unsafe { AtomicUsize::from_ptr(slot) }.swap(new_fn, Ordering::SeqCst);

    let mut unused = PAGE_PROTECTION_FLAGS(0);
    // SAFETY: `slot`/size match the first `VirtualProtect` call exactly, so
    // this restores the region's original protection.
    unsafe { VirtualProtect(slot.cast(), size_of::<usize>(), old_protect, &mut unused) }
        .expect("VirtualProtect (restore vtable slot protection) failed");

    original
}

/// Hooks for DirectX 11.
///
/// Deviates from upstream hudhook: instead of MinHook inline-patching the
/// `Present`/`ResizeBuffers` function bodies (which requires suspending
/// every other thread in the process to safely patch code — multi-second on
/// a heavily multithreaded target), this swaps the two function-pointer
/// slots in the shared `IDXGISwapChain` vtable directly via
/// `patch_vtable_slot`. `hooks()` returns an empty slice — no `MhHook` is
/// ever created — so `Hudhook::apply()`'s `MH_ApplyQueued()` call sees zero
/// pending hooks process-wide and skips MinHook's thread-freeze entirely.
pub struct ImguiDx11Hooks([(*mut usize, usize); 2]);

impl ImguiDx11Hooks {
    /// Construct the vtable hooks that will render UI via the provided
    /// [`ImguiRenderLoop`].
    ///
    /// The following functions are hooked:
    /// - `IDXGISwapChain::Present`
    /// - `IDXGISwapChain::ResizeBuffers`
    ///
    /// # Safety
    ///
    /// yolo
    pub unsafe fn new<T>(t: T) -> Self
    where
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        let (present_slot, resize_buffers_slot) = get_target_vtable_slots();

        trace!("IDXGISwapChain::Present slot = {:p}", present_slot);
        // SAFETY: `present_slot` was just resolved from a live swapchain's
        // vtable above.
        let present_original = unsafe {
            patch_vtable_slot(present_slot, dxgi_swap_chain_present_impl as *const () as usize)
        };

        trace!("IDXGISwapChain::ResizeBuffers slot = {:p}", resize_buffers_slot);
        // SAFETY: as above.
        let resize_buffers_original = unsafe {
            patch_vtable_slot(
                resize_buffers_slot,
                dxgi_swap_chain_resize_buffers_impl as *const () as usize,
            )
        };

        RENDER_LOOP.get_or_init(|| Box::new(t));
        TRAMPOLINES.get_or_init(|| Trampolines {
            dxgi_swap_chain_present: mem::transmute::<usize, DXGISwapChainPresentType>(
                present_original,
            ),
            dxgi_swap_chain_resize_buffers: mem::transmute::<usize, DXGISwapChainResizeBuffersType>(
                resize_buffers_original,
            ),
        });

        Self([(present_slot, present_original), (resize_buffers_slot, resize_buffers_original)])
    }
}

impl Hooks for ImguiDx11Hooks {
    fn from_render_loop<T>(t: T) -> Box<Self>
    where
        Self: Sized,
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        Box::new(unsafe { Self::new(t) })
    }

    fn hooks(&self) -> &[MhHook] {
        &[]
    }

    unsafe fn unhook(&mut self) {
        for (slot, original) in self.0 {
            // SAFETY: `slot` was resolved from a live swapchain vtable in
            // `new` and is still the same live, shared array.
            unsafe { patch_vtable_slot(slot, original) };
        }
        TRAMPOLINES.take();
        PIPELINE.take().map(|p| p.into_inner().take());
        RENDER_LOOP.take(); // should already be null
    }
}
