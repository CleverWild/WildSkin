//! Per-frame skin application: one-shot saved-choice apply plus every-frame
//! re-sync of hero transform stacks and minion/ward/jungle-mob skins.

use crate::sdk::ai_base_common::AIBaseCommon;
use crate::util::fnv::fnv1a;
use std::ffi::CString;
use std::sync::Once;

static CHANGE_SKINS_ONCE: Once = Once::new();

/// # Safety
/// Caller guarantees `obj` is a live `AIBaseCommon` in the game's memory and
/// `offsets` was resolved against that same live process.
unsafe fn change_skin_for_object(
    offsets: &crate::memory::ResolvedOffsets,
    obj: &AIBaseCommon,
    skin: i32,
) {
    if skin == -1 {
        return;
    }
    // SAFETY: per fn contract.
    let stack = unsafe { obj.character_data_stack_mut(offsets.fields.character_data_stack) };
    if stack.base_skin.skin != skin {
        stack.base_skin.skin = skin;
        // SAFETY: per fn contract.
        unsafe {
            stack.update(offsets.fns.skin_apply.update, true);
        }
    }
}

/// Applies saved skin choices once (`CHANGE_SKINS_ONCE`), then every frame
/// re-syncs each hero's transform-stack front and re-applies minion/ward/
/// jungle-mob skins (minions respawn, so only the champion choice is one-shot).
///
/// # Safety
/// Must run inside the target game process with `state::get()` initialized and
/// its offsets resolved against a live, `Running` game.
#[expect(
    clippy::significant_drop_tightening,
    reason = "`config` is read on every iteration of the minions loop; clippy's suggested drop point is inside that loop and fails to compile across iterations (E0382)"
)]
pub unsafe fn apply_frame() {
    let state = crate::state::get();
    let off = &state.offsets;
    let player = off.player();
    // SAFETY: per fn contract.
    let player_ref = unsafe { off.player_ref() };
    // SAFETY: per fn contract.
    let heroes = unsafe { off.hero_list() };
    // SAFETY: per fn contract.
    let minions = unsafe { off.minion_list() };

    let player_hash = player_ref.map_or(0, |p_ref| {
        // SAFETY: per fn contract.
        let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
        // SAFETY: per fn contract.
        fnv1a(unsafe { stack.base_skin.model.as_str() })
    });

    CHANGE_SKINS_ONCE.call_once(|| {
        let config = state.config.lock().unwrap();

        if let Some(p_ref) = player_ref
            && config.current_combo_skin_index > 0
        {
            // SAFETY: per fn contract.
            let stack = unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) };
            // SAFETY: per fn contract.
            let model = unsafe { stack.base_skin.model.as_str() }.to_owned();
            let hash = fnv1a(&model);
            if let Some(values) = state.database.champions_skins.get(&hash)
                && let Some(entry) = values.get((config.current_combo_skin_index - 1) as usize)
                && let Ok(c_model) = CString::new(entry.model_name.clone())
            {
                // SAFETY: per fn contract.
                unsafe {
                    p_ref.change_skin(
                        off.fields.character_data_stack,
                        off.fields.skin_id,
                        &off.fns.skin_apply,
                        &c_model,
                        entry.skin_id,
                        &state.database.special_skins,
                    );
                }
            }
        }

        let my_team = player_ref.map_or(1, |p_ref| p_ref.team);
        for &hero_ref in heroes {
            if player.map(|p| p as usize) == Some(std::ptr::from_ref(hero_ref) as usize) {
                continue;
            }
            let champ_hash = {
                // SAFETY: per fn contract.
                let stack =
                    unsafe { hero_ref.character_data_stack_ref(off.fields.character_data_stack) };
                // SAFETY: per fn contract.
                fnv1a(unsafe { stack.base_skin.model.as_str() })
            };
            if champ_hash == fnv1a("PracticeTool_TargetDummy") {
                continue;
            }

            let is_enemy = my_team != hero_ref.team;
            let idx = if is_enemy {
                config
                    .current_combo_enemy_skin_index
                    .get(&champ_hash)
                    .copied()
            } else {
                config
                    .current_combo_ally_skin_index
                    .get(&champ_hash)
                    .copied()
            };

            if let Some(idx) = idx
                && idx > 0
                && let Some(values) = state.database.champions_skins.get(&champ_hash)
                && let Some(entry) = values.get((idx - 1) as usize)
                && let Ok(c_model) = CString::new(entry.model_name.clone())
            {
                // SAFETY: per fn contract.
                unsafe {
                    hero_ref.change_skin(
                        off.fields.character_data_stack,
                        off.fields.skin_id,
                        &off.fns.skin_apply,
                        &c_model,
                        entry.skin_id,
                        &state.database.special_skins,
                    );
                }
            }
        }
    });

    for &hero_ref in heroes {
        // SAFETY: per fn contract.
        let stack = unsafe { hero_ref.character_data_stack_mut(off.fields.character_data_stack) };
        // SAFETY: per fn contract.
        if unsafe { stack.is_stack_empty() } {
            continue;
        }
        // SAFETY: per fn contract.
        let champ_hash = fnv1a(unsafe { stack.base_skin.model.as_str() });
        // Viego/Sylas transform into a 2nd-form champion; our skin id may not match.
        if champ_hash == fnv1a("Viego") || champ_hash == fnv1a("Sylas") {
            continue;
        }
        let base_skin_value = stack.base_skin.skin;
        // SAFETY: per fn contract; stack is non-empty, checked above.
        let front = unsafe { stack.stack_front_mut() };
        if front.skin != base_skin_value {
            front.skin = base_skin_value;
            // SAFETY: per fn contract.
            unsafe {
                stack.update(off.fns.skin_apply.update, true);
            }
        }
    }

    let config = state.config.lock().unwrap();
    let player_team = player_ref.map(|p_ref| p_ref.team);

    for &minion_ref in minions {
        // SAFETY: per fn contract.
        if unsafe { minion_ref.is_lane_minion() } && config.current_minion_skin_index != -1 {
            let skin = if player_team == Some(2) {
                config.current_minion_skin_index * 2 + 1
            } else {
                config.current_minion_skin_index * 2
            };
            // SAFETY: per fn contract.
            unsafe {
                change_skin_for_object(off, minion_ref, skin);
            }
            continue;
        }

        let hash = {
            // SAFETY: per fn contract.
            let stack =
                unsafe { minion_ref.character_data_stack_ref(off.fields.character_data_stack) };
            // SAFETY: per fn contract.
            fnv1a(unsafe { stack.base_skin.model.as_str() })
        };

        // SAFETY: per fn contract.
        let owner_ptr =
            unsafe { minion_ref.gold_redirect_target(off.fns.get_gold_redirect_target) };
        if !owner_ptr.is_null() {
            // SAFETY: a non-null `gold_redirect_target` result is a live
            // `AIBaseCommon`, per that fn's contract.
            let owner_ref = unsafe { &*owner_ptr };
            // SAFETY: per fn contract.
            let owner_skin =
                unsafe { owner_ref.character_data_stack_ref(off.fields.character_data_stack) }
                    .base_skin
                    .skin;

            let is_ward_like = hash == fnv1a("JammerDevice")
                || hash == fnv1a("SightWard")
                || hash == fnv1a("YellowTrinket")
                || hash == fnv1a("VisionWard")
                || hash == fnv1a("BlueTrinket")
                || hash == fnv1a("TestCubeRender10Vision");

            if is_ward_like {
                let is_local = player.is_none_or(|p| std::ptr::eq(p, owner_ptr));
                if is_local {
                    if hash == fnv1a("TestCubeRender10Vision") && player_hash == fnv1a("Yone") {
                        // SAFETY: per fn contract.
                        unsafe {
                            change_skin_for_object(off, minion_ref, owner_skin);
                        }
                    } else if hash == fnv1a("TestCubeRender10Vision") {
                        // SAFETY: per fn contract.
                        unsafe {
                            change_skin_for_object(off, minion_ref, 0);
                        }
                    } else {
                        // SAFETY: per fn contract.
                        unsafe {
                            change_skin_for_object(off, minion_ref, config.current_ward_skin_index);
                        }
                    }
                }
            } else if hash != fnv1a("SRU_Jungle_Companions") && hash != fnv1a("DominationScout") {
                // SAFETY: per fn contract.
                unsafe {
                    change_skin_for_object(off, minion_ref, owner_skin);
                }
            }
            continue;
        }

        if let Some(&idx) = config.current_combo_jungle_mob_skin_index.get(&hash)
            && idx != 0
        {
            // SAFETY: per fn contract.
            unsafe {
                change_skin_for_object(off, minion_ref, idx - 1);
            }
            continue;
        }

        if let Some(p_ref) = player_ref {
            // Companion skins (Nunu snowball, Kindred wolf, Quinn Valor) follow
            // the local player's skin.
            let matches = (hash == fnv1a("NunuSnowball") && player_hash == fnv1a("Nunu"))
                || (hash == fnv1a("KindredWolf") && player_hash == fnv1a("Kindred"))
                || (hash == fnv1a("QuinnValor") && player_hash == fnv1a("Quinn"));
            if matches {
                // SAFETY: per fn contract.
                let player_skin =
                    unsafe { p_ref.character_data_stack_ref(off.fields.character_data_stack) }
                        .base_skin
                        .skin;
                // SAFETY: per fn contract.
                unsafe {
                    change_skin_for_object(off, minion_ref, player_skin);
                }
            }
        }
    }
}
