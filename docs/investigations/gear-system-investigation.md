# Investigation: auto-detecting "special skin" gear variants

**TL;DR:** No — confirmed via static analysis of the game binary.
`CharacterDataStack::Update` forwards the `gear` byte to a rendering vtable
call completely unvalidated: no clamp, no range check, no lookup table
anywhere in the executable. Gear-variant validity is asset-existence-driven
(whether a matching resource file exists on disk), not something readable
from process memory. Keep `special_skins` hardcoded.

## Goal

`SkinDatabase::empty()` hardcodes an 11-entry `special_skins` table
([skin_database.rs:93-105](../../WildSkin-rs/src/skin_database.rs#L93-L105))
— champions whose skin has selectable "gear" sub-variants (Katarina's dagger
colors, Renekton's forms, etc.), each with a manually curated ID range and
label list. Question: can this be auto-detected at runtime, the way
`champions_skins`/`wards_skins` already are (loaded live from
`ChampionManager` + the game's own `translate_string`)?

Short answer: not with the tools/access available. Findings below so a
future attempt doesn't re-derive them.

## What already works, for contrast

`champions_skins`/`wards_skins` work because there's a direct, enumerable
path: `ChampionManager::champions[i]->skins[]` gives real skin IDs, and
`translate_string(...)` gives the localized name — both reachable in live
process memory.

## What doesn't have an equivalent

No such path exists for gear:

- No literal label strings ("Sahn-Uzal", "Dagger", etc.) anywhere in the
  executable — expected if they're translated, but rules out reading them
  directly.
- Every `game_character_*` translation-key template in the binary was
  enumerated — none reference gear.
- Two `"Gear_%u"`/`"Gear%u"` format strings exist, but trace to UI
  event-binding keys (click handlers for gear buttons), not translation
  lookups.

## The real "Gear" subsystem

Type-name strings confirm genuine engine classes (`GearComponent`,
`GearSelectionViewController`, a `PKT_S2C_EquipGear_s` network packet) —
"gear" is a real engine feature. Tracing the UI population chain shows gear
counts are populated through a generic view-controller pattern consistent
with lobby/collection/store UI (pre-match skin customization), not the live
in-match object this DLL has a pointer to. The live in-match packet handler
could not be located statically.

## Confirmed: `CharacterDataStack::Update` performs no validation

Decompiling the function directly: it reads the `gear` byte (sentinel value
meaning "no override") and, if set, forwards it unconditionally to a vtable
call — no clamp, no range check, no count table:

```c
rsi = zx.q(*(r14_1 + 0x84))
if (rsi.b != 0xff)
    (*(*rcx_27 + 0x38))(rcx_27, zx.q(rsi.b), 0)  // forward blindly
```

Confirms the asset-driven hypothesis: the valid gear range isn't stored as a
count anywhere in memory — it's implicit in which resource files exist in
the game's asset bundles.

## Why this stalled

- No recovered C++ types for the relevant classes exist in the tooling used
  — every struct layout above was inferred from decompiled pointer
  arithmetic, not a real type.
- The alternative (parsing the game's asset archives from disk for resource
  existence) is a different scope entirely from process-memory reading, and
  is the actual mechanism.

## Call-graph walk: no count/validation found anywhere upstream

Walked every caller of `CharacterDataStack::Update` up several layers
(through generic per-unit network-field-apply dispatchers). All layers
examined are structurally identical: look up an entry, copy already-supplied
fields, call `Update`/refresh UI — none contain a range check, modulo, or
comparison against a per-skin/per-champion constant. `CharacterDataStack`
lives at a fixed offset inside the network-facing per-unit object, applied
through the same uniform field-apply mechanism as every other network field
— gear included, with no special-cased validation.

One dead end: a virtual-call gate that's the closest thing to a "does this
unit have gear" boolean couldn't be resolved statically (no recovered type
for the object, so the vtable target is unknown) — would need dynamic
analysis (breakpoint while playing a gear-enabled champion) to pin down.

## Searched for hardcoded gear-count constants directly

Searched for literal numeric/hash constants the engine might compare
against (a known skin ID as an immediate compare, and the crate's own
FNV-1a hash of known champion/model names): zero hits, for every variant
tried. Expected, given the above — if the engine never checks "does skin N
support gear," there's no comparison to find. Also confirms the crate's own
FNV-1a hashing scheme is the original mod author's own invented convention,
not something matching an internal engine hash.

This also reframes the original question: it's not that ~11 champions have
a bespoke "gear system" bolted on — every skin can potentially carry
gear-variant assets, and the ~11 the mod hardcodes are just the ones that
happen to ship them.

## Side-finding (unrelated to gear, worth keeping)

A pre-existing code annotation flagged a suspected memory-leak/crash risk in
the Rust port's stack-clearing logic (`clear_stack()` never running the
string destructor, allegedly corrupting heap buffers across repeated
`push()` calls — suspected cause of skin-switch rendering artifacts).
Verified against current source: the only call site of the leaky
`clear_stack()` is in a unit test; the live skin-change path already uses
`clear_stack_properly()` (which does run the destructor). **Already fixed,
not a live bug** — worth remembering if this old warning resurfaces.

## Recommendation

Keep `special_skins` hardcoded. This is a confirmed "nothing in live
process memory to find," not a "couldn't find it": `Update()` shows no
validation/count logic for `gear` at the one place it would have to exist,
and every call-graph layer examined converges on the same pass-through
shape. The only way to get this data programmatically would be parsing the
game's asset archives from disk for resource existence — out of scope for a
process-memory-reading DLL.
