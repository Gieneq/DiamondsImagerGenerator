use std::fmt::Debug;
use millimeter::{
    mm, 
    Unit
};

#[derive(Debug, Clone, Copy)]
pub struct Size2X<T> 
{
    pub w: T,
    pub h: T,
}

impl<T> Size2X<T> 
where 
    T: Into<f32> + Copy + PartialEq
{
    pub fn get_aspect_ratio(&self) -> f32 {
        self.w.into() / self.h.into()
    }

    pub fn is_horizontal(&self) -> bool {
        self.get_aspect_ratio() > 1.0
    }

    pub fn is_vertical(&self) -> bool {
        self.get_aspect_ratio() < 1.0
    }

    pub fn is_square(&self) -> bool {
        self.w == self.h
    }
}

impl<T> Size2X<T>
where 
    T: Copy
{
    pub fn new_square(side: T) -> Self {
        Self { w: side, h: side }
    }
}

pub type Size2D = Size2X<mm>;
pub type Size2U = Size2X<u32>;
pub type Size2F = Size2X<f32>;

impl From<&Size2D> for Size2F {
    fn from(value: &Size2D) -> Self {
        Self {
            w: value.w.raw_value(),
            h: value.h.raw_value()
        }
    }
}

impl From<&Size2U> for Size2F {
    fn from(value: &Size2U) -> Self {
        Self {
            w: value.w as f32,
            h: value.h as f32
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pos2D {
    pub x: mm,
    pub y: mm,
}

#[derive(Debug, Clone, Copy)]
pub struct PaperSheet {
    pub size: Size2D,
    pub print_margins: MarginsMirrored2D
}

#[derive(Debug, Clone, Copy)]
pub struct MarginsMirrored2D {
    pub vertical: mm,
    pub horizontal: mm,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect2D {
    pub pos: Pos2D,
    pub size: Size2D,
}

#[derive(Debug, Clone, Copy)]
pub enum DiamondShape {
    Round {
        diameter: mm
    },
    Square {
        side: mm
    }
}

impl DiamondShape {
    pub fn get_size(&self) -> mm {
        match self {
            DiamondShape::Round{diameter} => *diameter,
            DiamondShape::Square{side} => *side,
        }
    }
}

impl From<Rect2D> for (f32, f32, f32, f32) {
    fn from(value: Rect2D) -> Self {
        (
            value.left().raw_value(),
            value.bottom().raw_value(),
            value.size.w.raw_value(),
            value.size.h.raw_value()
        )
    }
}

impl From<Rect2D> for (mm, mm, mm, mm) {
    fn from(value: Rect2D) -> Self {
        (
            value.left(),
            value.bottom(),
            value.size.w,
            value.size.h
        )
    }
}

impl MarginsMirrored2D {
    pub fn swap_v_h(&mut self) {
        std::mem::swap(&mut self.vertical, &mut self.horizontal);
    }
}

impl Size2D {
    pub fn swap_w_h(&mut self) {
        std::mem::swap(&mut self.w, &mut self.h);
    }
}

impl Rect2D {
    pub fn top(&self) -> mm {
        self.bottom() + self.size.h
    }
    
    pub fn right(&self) -> mm {
        self.left() + self.size.w
    }
    
    pub fn bottom(&self) -> mm {
        self.pos.y
    }
    
    pub fn left(&self) -> mm {
        self.pos.x
    }
    
    pub fn get_centered(&self, size_to_be_centered: &Size2D) -> Self {
        let x_offset = (self.size.w - size_to_be_centered.w) / 2.0;
        let y_offset = (self.size.h - size_to_be_centered.h) / 2.0;

        Self {
            pos: Pos2D {
                x: self.pos.x + x_offset,
                y: self.pos.y + y_offset
            },
            size: *size_to_be_centered
        }
    }
}

impl DiamondShape {
    pub fn common_round() -> Self {
        DiamondShape::Round { diameter: 2.8.mm() }
    }

    pub fn common_square() -> Self {
        DiamondShape::Square { side: 2.5.mm() }
    }
}

impl PaperSheet {
    pub fn change_orientation(&mut self) {
        self.size.swap_w_h();
        self.print_margins.swap_v_h();
    }

    pub fn get_printing_area_rect(&self) -> Rect2D {
        Rect2D {
            pos: Pos2D { 
                x: self.print_margins.horizontal, 
                y: self.print_margins.vertical,
            },
            size: Size2D {
                w: self.size.w - 2.0 * self.print_margins.horizontal,
                h: self.size.h - 2.0 * self.print_margins.vertical,
            }
        }
    }

    pub fn standard_a4() -> Self {
        Self {
            size: Size2D {
                w: 210.0.mm(),
                h: 297.0.mm()
            },
            print_margins: MarginsMirrored2D {
                vertical: 6.0.mm(),
                horizontal: 6.0.mm()
            }
        }
    }

    pub fn standard_a3() -> Self {
        Self {
            size: Size2D {
                w: 297.0.mm(),
                h: 420.0.mm()
            },
            print_margins: MarginsMirrored2D {
                vertical: 8.0.mm(),
                horizontal: 8.0.mm()
            }
        }
    }
}

