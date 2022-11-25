use std::collections::HashMap;


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LayoutBox {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[allow(dead_code)] // TEMP
impl LayoutBox {
    #[inline]
    fn check_size(size: [f32; 2]) {
        // TODO: is this correct?
        assert!(size[0] >= 0.0);
        assert!(size[1] >= 0.0);
    }

    #[inline]
    fn check_size_finite(size: [f32; 2]) {
        // TODO: is this correct?
        assert!(size[0] >= 0.0 && size[0] < f32::INFINITY);
        assert!(size[1] >= 0.0 && size[1] < f32::INFINITY);
    }


    #[inline]
    pub fn min_size(min: [f32; 2]) -> LayoutBox {
        Self::check_size_finite(min);
        LayoutBox { min, max: [f32::INFINITY, f32::INFINITY] }
    }

    #[inline]
    pub fn max_size(max: [f32; 2]) -> LayoutBox {
        Self::check_size(max);
        LayoutBox { min: [0.0, 0.0], max }
    }

    #[inline]
    pub fn tight(size: [f32; 2]) -> LayoutBox {
        Self::check_size_finite(size);
        LayoutBox { min: size, max: size }
    }

    #[inline]
    pub fn any() -> LayoutBox {
        LayoutBox { min: [0.0, 0.0], max: [f32::INFINITY, f32::INFINITY] }
    }

    #[inline]
    pub fn with_max(self, max: [f32; 2]) -> LayoutBox {
        Self::check_size(max);
        LayoutBox { min: self.min, max }
    }

    #[inline]
    pub fn clamp(self, size: [f32; 2]) -> [f32; 2] {
        [size[0].clamp(self.min[0], self.max[0]),
         size[1].clamp(self.min[1], self.max[1])]
    }

    #[inline]
    pub fn clamp_axis(self, size: f32, axis: usize) -> f32 {
        size.clamp(self.min[axis], self.max[axis])
    }

    #[inline]
    pub fn clamp_width(self, size: f32) -> f32 {
        self.clamp_axis(size, 0)
    }

    #[inline]
    pub fn clamp_height(self, size: f32) -> f32 {
        self.clamp_axis(size, 1)
    }


    #[inline]
    pub fn axis_is_tight(self, axis: usize) -> bool {
        self.min[axis] == self.max[axis]
    }

    #[inline]
    pub fn width_is_tight(self) -> bool {
        self.axis_is_tight(0)
    }

    #[inline]
    pub fn height_is_tight(self) -> bool {
        self.axis_is_tight(1)
    }
}



pub type Style = HashMap<String, String>;


pub enum Display {
    None,
    Inline,
    Block,
}


pub enum Layout {
    Lines,
}


pub enum Cursor {
    Default,
    Pointer,
    Text,
}

