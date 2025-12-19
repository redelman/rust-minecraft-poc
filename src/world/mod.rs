mod chunk;
mod terrain;
pub mod mesh_gen;

pub use chunk::{Chunk, ChunkCoord, CHUNK_SIZE, MAX_LIGHT_LEVEL};
pub use terrain::{ChunkManager, TerrainChunk, setup_terrain, spawn_chunks_around_player, process_chunk_tasks, get_spawn_height};
