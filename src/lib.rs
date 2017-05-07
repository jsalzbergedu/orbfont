#![crate_name="orbfont"]
#![crate_type="lib"]

extern crate orbclient;
extern crate rusttype;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use orbclient::{Color, Renderer};

pub struct Font {
    inner: rusttype::Font<'static>
}

impl Font {
    /// Find a font from an optional type, family, and style, such as "Mono", "Fira", "Regular"
    #[cfg(target_os = "redox")]
    pub fn find(typeface: Option<&str>, family: Option<&str>, style: Option<&str>) -> Result<Font, String> {
        Font::from_path(&format!("/ui/fonts/{}/{}/{}.ttf", typeface.unwrap_or("Mono"), family.unwrap_or("Fira"), style.unwrap_or("Regular")))
    }

    #[cfg(not(target_os = "redox"))]
    pub fn find(typeface: Option<&str>, family: Option<&str>, style: Option<&str>) -> Result<Font, String> {
        Font::from_path(&format!("/usr/share/fonts/truetype/liberation/{}{}-{}.ttf", family.unwrap_or("Liberation"), typeface.unwrap_or("Mono"), style.unwrap_or("Regular")))
    }

    /// Load a font from file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Font, String> {
        let mut file = try!(File::open(path).map_err(|err| format!("failed to open font: {}", err)));
        let mut data = Vec::new();
        let _ = try!(file.read_to_end(&mut data).map_err(|err| format!("failed to read font: {}", err)));
        Font::from_data(data)
    }

    /// Load a font from a slice
    pub fn from_data<D: Into<rusttype::SharedBytes<'static>>>(data: D) -> Result<Font, String> {
        let collection = rusttype::FontCollection::from_bytes(data);
        let font = try!(collection.into_font().ok_or("font collection did not have exactly one font".to_string()));

        Ok(Font {
            inner: font
        })
    }

    /// Render provided text using the font
    pub fn render<'a>(&'a self, text: &str, height: f32) -> Text<'a> {
        let scale = rusttype::Scale::uniform(height);

        // The origin of a line of text is at the baseline (roughly where non-descending letters sit).
        // We don't want to clip the text, so we shift it down with an offset when laying it out.
        // v_metrics.ascent is the distance between the baseline and the highest edge of any glyph in
        // the font. That's enough to guarantee that there's no clipping.
        let v_metrics = self.inner.v_metrics(scale);
        let offset = rusttype::point(0.0, v_metrics.ascent);

        // Glyphs to draw for "RustType". Feel free to try other strings.
        let glyphs: Vec<rusttype::PositionedGlyph> = self.inner.layout(text, scale, offset).collect();

        // Find the most visually pleasing width to display
        let width = glyphs.iter().rev()
            .filter_map(|g| g.pixel_bounding_box()
                        .map(|b| b.min.x as f32 + g.unpositioned().h_metrics().advance_width))
            .next().unwrap_or(0.0);

        Text {
            w: width.ceil() as u32,
            h: height.ceil() as u32,
            glyphs: glyphs
        }
    }
}

pub struct Text<'a> {
    w: u32,
    h: u32,
    glyphs: Vec<rusttype::PositionedGlyph<'a>>
}

impl<'a> Text<'a> {
    /// Return width of the text
    pub fn width(&self) -> u32 {
        self.w
    }

    /// Return height of the text
    pub fn height(&self) -> u32 {
        self.h
    }

    /// Draw the text onto a window
    pub fn draw<R: Renderer>(&self, renderer: &mut R, x: i32, y: i32, color: Color) {
        for g in self.glyphs.iter() {
            if let Some(bb) = g.pixel_bounding_box() {
                g.draw(|off_x, off_y, v| {
                    let off_x = off_x as i32 + bb.min.x;
                    let off_y = off_y as i32 + bb.min.y;
                    // There's still a possibility that the glyph clips the boundaries of the bitmap
                    if off_x >= 0 && off_x < self.w as i32 && off_y >= 0 && off_y < self.h as i32 {
                        let c = (v * 255.0) as u32;
                        renderer.pixel(x + off_x, y + off_y, Color{
                            data: c << 24 | (color.data & 0xFFFFFF)
                        });
                    }
                });
            }
        }
    }
}
