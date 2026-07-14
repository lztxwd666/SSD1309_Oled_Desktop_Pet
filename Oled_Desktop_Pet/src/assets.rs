//! 编译期精灵资源 —— build.rs 从 assets/images/ferris.png 生成。
//!
//! 常量：
//! * `BASE_SPRITE` — 64×64 单色帧缓冲区（512 字节），Bayer 4×4 抖动
//! * `SPRITE_W` / `SPRITE_H` — 精灵在 56×56 宠物区内的实际尺寸

// 由 build.rs 生成
include!(concat!(env!("OUT_DIR"), "/sprites.rs"));
