use std::fmt::Debug;

// Color Pool => [Red, Purple, Cyan, Green, Gray]
const DARK_COLOR_POOL: [&'static str; 5] = ["#ff0000", "#D49BF8", "#00FFFF", "#90EE90", "#B0B0B0"];
const LIGHT_COLOR_POOL: [&'static str; 5] = ["#AD0000", "#780DBA", "#006161", "#0F610F", "#474747"];
const DARK_BG_TEXT_COLOR: &'static str = "#ffffff";
const LIGHT_BG_TEXT_COLOR: &'static str = "#000000";

pub fn to_argb(color: &String) -> String {
    color.replace("#", "FF")
}

pub trait CellColorProfile: Debug {
    /// Default text color that looks good on the default background color
    fn get_default_text_color(&self) -> String;
    /// Default background color
    fn get_background_color(&self) -> String;
    /// Return other text colors that look good on the default background
    fn get_color(&mut self) -> String;
}

#[derive(Debug)]
pub struct White {
    pub color_pool_pos: usize,
}

impl CellColorProfile for White {
    fn get_default_text_color(&self) -> String {
        LIGHT_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#ffffff".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..LIGHT_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = LIGHT_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}

#[derive(Debug)]
pub struct Yellow {
    pub color_pool_pos: usize,
}

impl CellColorProfile for Yellow {
    fn get_default_text_color(&self) -> String {
        LIGHT_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#ffff00".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..LIGHT_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = LIGHT_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}

#[derive(Debug)]
pub struct Beige {
    pub color_pool_pos: usize,
}

impl CellColorProfile for Beige {
    fn get_default_text_color(&self) -> String {
        LIGHT_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#F5F5DC".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..LIGHT_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = LIGHT_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}

#[derive(Debug)]
pub struct Lavender {
    pub color_pool_pos: usize,
}

impl CellColorProfile for Lavender {
    fn get_default_text_color(&self) -> String {
        LIGHT_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#E6E6FA".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..LIGHT_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = LIGHT_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}

#[derive(Debug)]
pub struct Black {
    pub color_pool_pos: usize,
}

impl CellColorProfile for Black {
    fn get_default_text_color(&self) -> String {
        DARK_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#000000".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..DARK_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = DARK_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}

#[derive(Debug)]
pub struct NavyBlue {
    pub color_pool_pos: usize,
}

impl CellColorProfile for NavyBlue {
    fn get_default_text_color(&self) -> String {
        DARK_BG_TEXT_COLOR.into()
    }

    fn get_background_color(&self) -> String {
        "#000080".into()
    }

    fn get_color(&mut self) -> String {
        if !(0..DARK_COLOR_POOL.len()).contains(&self.color_pool_pos) {
            self.color_pool_pos = 0;
        }
        let res = DARK_COLOR_POOL[self.color_pool_pos].into();
        self.color_pool_pos += 1;
        res
    }
}
