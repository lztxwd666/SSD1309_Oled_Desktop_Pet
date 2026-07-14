//! 屏幕布局引擎 —— 定义 128×64 显示区域划分。

#[derive(Debug, Clone, Copy)]
pub struct Rect { pub x: usize, pub y: usize, pub w: usize, pub h: usize }
impl Rect {
    #[inline] pub fn right(&self) -> usize { self.x + self.w }
    #[inline] pub fn bottom(&self) -> usize { self.y + self.h }
}

pub struct Layout { pub pet: Rect, pub divider_x: usize, pub metrics: Rect, pub status: Rect }

/// 全局布局：桌宠 56×56，指标 70×56，状态栏 128×7。
///
/// 指标面板 70px × 5px/字 = 14 字符。所有格式化函数经测试保证 ≤14 字符。
pub const LAYOUT: Layout = Layout {
    pet:       Rect { x: 0,  y: 0,  w: 56, h: 56 },
    divider_x: 56,
    metrics:   Rect { x: 58, y: 0,  w: 70, h: 56 },
    status:    Rect { x: 0,  y: 57, w: 128, h: 7 },
};

pub const ROW_H: usize = 8;
/// 组间空白像素（2px 紧凑间距，确保新指标行不溢出 56px 面板高度）。
pub const GROUP_GAP: usize = 2;
