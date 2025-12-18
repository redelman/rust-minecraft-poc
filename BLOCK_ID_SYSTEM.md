# Block ID System and Mod Safety

## The Problem

You've identified a critical issue: **What happens when mods are removed or blocks are deleted?**

If we just assign numeric IDs in registration order:
1. Session 1: `dirt=1`, `stone=2`, `grass=3`
2. User removes a mod that added "stone"
3. Session 2: `dirt=1`, `grass=2` ← **grass changed from ID 3 to ID 2!**
4. All saved grass blocks would now load as the wrong block type

This is exactly the problem old Minecraft had with numeric block IDs.

## Our Solution

We use a **hybrid system** that gives us the best of both worlds:

### 1. String IDs for Persistence (Mod-Safe)
- Blocks are registered with string IDs like `"core:dirt"`, `"mymod:custom_block"`
- **Worlds are saved with string IDs**, not numeric ones
- String IDs are globally unique (namespace prevents conflicts)

### 2. Numeric IDs for Runtime (Memory-Efficient)
- At runtime, chunks store `BlockId(u16)` - only 2 bytes per block
- 16x16x16 chunk = 4,096 blocks × 2 bytes = 8 KB per chunk
- Compare to storing strings: 4,096 × ~20 bytes = ~80 KB per chunk (10x more!)

### 3. Registry Maintains the Mapping
- The `BlockRegistry` maps string IDs ↔ numeric IDs
- Numeric IDs can change between sessions - **that's okay!**
- When loading a world, we remap: `"core:dirt"` → whatever numeric ID it has this session

## How It Works

### Saving a World
```rust
// Pseudocode for world save
for block_id in chunk.blocks {
    let string_id = registry.get_string_id(block_id);  // BlockId(5) → "core:dirt"
    save_to_file(string_id);  // Save "core:dirt" to disk
}
```

### Loading a World
```rust
// Pseudocode for world load
for string_id in load_from_file() {
    // "core:dirt" in file
    let block_id = registry.get_id(string_id);  // "core:dirt" → BlockId(7) this session
    chunk.blocks.push(block_id);
}
```

## Handling Edge Cases

### Case 1: Mod Removed (Block No Longer Exists)
```
Saved world has: "coolmod:laser_block"
Current session: CoolMod not installed

Solution: Replace with AIR or a "Missing Block" placeholder
```

### Case 2: Mod Updated (Block Removed)
```
Saved world has: "mymod:old_block"
Current session: MyMod v2.0 removed "old_block"

Solution: Same as Case 1 - replace with AIR
```

### Case 3: Block ID Registration Order Changes
```
Session 1: dirt=1, stone=2, grass=3
Session 2: grass=1, dirt=2, stone=3  ← Order changed

Result: No problem! Worlds save "core:dirt" not "1"
```

## Implementation TODO

To implement this, we need to add:

1. **String ID Reverse Lookup** (already partially there)
   ```rust
   impl BlockRegistry {
       pub fn get_string_id(&self, id: BlockId) -> Option<&str> { /* ... */ }
   }
   ```

2. **World Serialization** (future)
   - Save chunks with string IDs
   - Save the full registry mapping for validation

3. **World Deserialization** (future)
   - Load string IDs from file
   - Remap to current session's numeric IDs
   - Handle missing blocks gracefully

4. **Missing Block Handling**
   ```rust
   match registry.get_id("coolmod:laser_block") {
       Some(id) => id,
       None => {
           warn!("Block 'coolmod:laser_block' not found, replacing with AIR");
           BlockId::AIR
       }
   }
   ```

## Why This Is Better Than Minecraft's Old System

Minecraft originally used:
- **Hardcoded numeric IDs** - mods would conflict if they used the same number
- **Mod config files** - users had to manually fix ID conflicts
- **World corruption** - if IDs changed, blocks would transform

Our system:
- **Namespaced string IDs** - impossible to have conflicts (`core:dirt` vs `mymod:dirt`)
- **No manual configuration** - everything is automatic
- **Graceful degradation** - missing blocks become AIR, don't corrupt the world
- **Memory efficient** - still only 2 bytes per block at runtime

## Current Status

✅ String ID registration system (mod-safe)
✅ Numeric ID runtime storage (memory-efficient)
✅ Registry maintains mapping
❌ World save/load not implemented yet (that's okay for now)
❌ Missing block handling not implemented yet

When we implement world persistence, we'll add the save/load logic that uses string IDs.
