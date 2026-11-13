use crate::screen::viewport;

pub type Bitmap = u32;

#[derive(Clone, Copy)]
pub enum Font {
    Small = 0,
    Normal = 1,
    NormalCenter = 2,
    Big = 3,
    TitleMid = 6,
}

#[link(wasm_import_module = "env")]
extern "C" {
    fn canvas_begin(scale: f32, red: f32, green: f32, blue: f32);
    fn canvas_origin(scale: f32, x: f32, y: f32);
    fn canvas_rect(x: f32, y: f32, w: f32, h: f32, stroke: f32, r: f32, g: f32, b: f32, a: f32);
    fn canvas_circle(x: f32, y: f32, radius: f32, r: f32, g: f32, b: f32, a: f32);
    fn canvas_text(
        ptr: *const u8,
        len: usize,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        font: u32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    fn canvas_bitmap(w: u32, h: u32, pixels: *const u32) -> Bitmap;
    fn canvas_sprite(id: Bitmap, x: f32, y: f32, w: f32, h: f32, alpha: f32);
    fn browser_fullscreen();
}

pub struct Gfx {
    pub ui: f32,
    pub width: u32,
    pub height: u32,
    pub d2d_gen: u32,
    clear: [f32; 4],
}

impl Gfx {
    pub fn new(w: u32, h: u32) -> Self {
        let (ui, width, height) = viewport(w, h);
        Self {
            ui,
            width,
            height,
            d2d_gen: 1,
            clear: [0.0; 4],
        }
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        (self.ui, self.width, self.height) = viewport(w, h);
    }

    pub fn toggle_fullscreen(&mut self) {
        unsafe { browser_fullscreen() }
    }

    pub fn begin(&mut self, clear: [f32; 4]) {
        self.clear = clear;
    }

    pub fn d2d_begin(&mut self) -> bool {
        let [r, g, b, _] = self.clear;
        unsafe { canvas_begin(self.ui, r, g, b) };
        true
    }

    pub fn set_origin(&self, x: f32, y: f32) {
        unsafe { canvas_origin(self.ui, x, y) };
    }

    pub fn d2d_end(&mut self) {}
    pub fn present(&mut self) {}

    pub fn fill_rect(&self, x: f32, y: f32, right: f32, bottom: f32, [r, g, b, a]: [f32; 4]) {
        unsafe { canvas_rect(x, y, right - x, bottom - y, 0.0, r, g, b, a) }
    }

    pub fn stroke_rect(
        &self,
        x: f32,
        y: f32,
        right: f32,
        bottom: f32,
        width: f32,
        [r, g, b, a]: [f32; 4],
    ) {
        unsafe { canvas_rect(x, y, right - x, bottom - y, width, r, g, b, a) }
    }

    pub fn fill_circle(&self, x: f32, y: f32, radius: f32, [r, g, b, a]: [f32; 4]) {
        unsafe { canvas_circle(x, y, radius, r, g, b, a) }
    }

    pub fn text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        font: Font,
        [r, g, b, a]: [f32; 4],
    ) {
        unsafe {
            canvas_text(
                text.as_ptr(),
                text.len(),
                x,
                y,
                w,
                h,
                font as u32,
                r,
                g,
                b,
                a,
            )
        }
    }

    pub fn create_bitmap(&self, w: u32, h: u32, pixels: &[u32]) -> Option<Bitmap> {
        Some(unsafe { canvas_bitmap(w, h, pixels.as_ptr()) })
    }

    pub fn draw_bitmap_alpha(
        &self,
        bitmap: &Bitmap,
        x: f32,
        y: f32,
        right: f32,
        bottom: f32,
        alpha: f32,
    ) {
        unsafe { canvas_sprite(*bitmap, x, y, right - x, bottom - y, alpha) }
    }
}
