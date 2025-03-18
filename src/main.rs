use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use pdf_canvas::{Pdf, BuiltinFont};
use pdf_canvas::graphicsstate::Color;
use serde::{Deserialize, Serialize};
use image::Rgb;

#[derive(Debug, Clone, Copy)]
pub enum PaperSize {
    VerticalA4,
    VerticalA3,
}

#[derive(Debug, Clone, Copy)]
pub struct PrintMargins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Size2F {
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Size2U {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Pos2F {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect2F {
    pub pos: Pos2F,
    pub size: Size2F,
}

impl From<Rect2F> for (f32, f32, f32, f32) {
    fn from(value: Rect2F) -> Self {
        (
            value.left(),
            value.bottom(),
            value.size.w,
            value.size.h
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette (Vec<[u8; 3]>);

// Will check pixels vs palette and break if invalid found
#[derive(Debug, Clone)]
pub struct GenerateSettings {
    pub paper_size: PaperSize,
    pub image_size: Size2F,
    pub palette: Palette,
}

impl PrintMargins {
    pub fn get_vertical_margins(&self) -> f32 {
        self.top + self.bottom
    }
    
    pub fn get_horizontal_margins(&self) -> f32 {
        self.left + self.right
    }
}

impl Rect2F {
    pub fn top(&self) -> f32 {
        self.bottom() + self.size.h
    }
    
    pub fn right(&self) -> f32 {
        self.left() + self.size.w
    }
    
    pub fn bottom(&self) -> f32 {
        self.pos.y
    }
    
    pub fn left(&self) -> f32 {
        self.pos.x
    }
}

impl PaperSize {
    pub fn get_paper_size(&self) -> Size2F {
        match self {
            Self::VerticalA4 => Size2F { w: 210.0, h: 297.0 },
            Self::VerticalA3 => Size2F { w: 297.0, h: 420.0 },
        }
    }
    
    pub fn get_drawable_rect(&self) -> Rect2F {
        let paper_size = self.get_paper_size();
        let print_margins = self.get_printing_margins();
        Rect2F {
            pos: Pos2F { 
                x: print_margins.left, 
                y: print_margins.bottom,
            },
            size: Size2F {
                w: paper_size.w - print_margins.get_horizontal_margins(),
                h: paper_size.h - print_margins.get_vertical_margins(),
            }
        }
    }
    
    pub fn get_printing_margins(&self) -> PrintMargins {
        match self {
            Self::VerticalA4 => PrintMargins {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 8.0,
            },
            Self::VerticalA3 => PrintMargins {
                top: 10.0,
                right: 10.0,
                bottom: 10.0,
                left: 10.0,
            },
        }
    }
}

fn is_size_valid(
    painting_size: &Size2F,
    image_size: &Size2U,
    diamonds_step: f32,
) -> bool {
    (image_size.w as f32 * diamonds_step < painting_size.w) 
        && (image_size.h as f32 * diamonds_step < painting_size.h)
}

fn get_painting_actual_rect(
    diamonds_painting_workspace_rect: &Rect2F,
    image_size: &Size2U,
    diamonds_step: f32
) -> Result<Rect2F, ()> {
    if !is_size_valid(&diamonds_painting_workspace_rect.size, image_size, diamonds_step) {
        return Err(())
    }

    let actual_painting_size = Size2F {
        w: image_size.w as f32 * diamonds_step,
        h: image_size.h as f32 * diamonds_step,
    };

    Ok(Rect2F {
        pos: Pos2F {
            x: diamonds_painting_workspace_rect.pos.x + (diamonds_painting_workspace_rect.size.w - actual_painting_size.w) / 2.0,
            y: diamonds_painting_workspace_rect.pos.y + (diamonds_painting_workspace_rect.size.h - actual_painting_size.h) / 2.0
        },
        size: actual_painting_size
    })
}

fn whiten_u8(src_channel: u8, norm_whiteness: f32) -> u8 {
    (src_channel as f32 * (1.0 - norm_whiteness) + 255.0 * norm_whiteness).round().clamp(0.0, 255.0) as u8
}

fn whiten_pixel(src_pixel: &Rgb<u8>, norm_whiteness: f32) -> Rgb<u8> {
    Rgb([
        whiten_u8(src_pixel.0[0], norm_whiteness),
        whiten_u8(src_pixel.0[1], norm_whiteness),
        whiten_u8(src_pixel.0[2], norm_whiteness)
    ])
}

pub fn generate_project_pdf<P: AsRef<Path>>(
    src_img_path: P,
    src_palette_json: P,
    paper_size: PaperSize,
    diamond_diameter: f32,
    diamonds_spacing: f32,
    output_pdf_path: &str,
) {
    let diamonds_step = diamond_diameter + diamonds_spacing;

    let img = image::open(src_img_path).unwrap();
    let image_size = Size2U {
        w: img.width(),
        h: img.height()
    };

    let palette: Palette = {
        let file = File::open(src_palette_json).unwrap();
        let file_reader = BufReader::new(file);
        serde_json::from_reader(file_reader).unwrap()
    };

    const LEGEND_LINE_HEIGHT: f32 = 6.0;
    const LEGEND_LINE_MARGINS: f32 = 1.5;
    let legend_lines_count = palette.0.len();

    let legend_area_rect = Rect2F {
        pos: Pos2F {
            x: paper_size.get_drawable_rect().left(),
            y: paper_size.get_drawable_rect().bottom()
        },
        size: Size2F {
            w: paper_size.get_drawable_rect().size.w,
            h: legend_lines_count as f32 * LEGEND_LINE_HEIGHT
        }
    };

    let diamonds_painting_workspace_rect = Rect2F {
        pos: Pos2F {
            x: legend_area_rect.left(),
            y: legend_area_rect.top()
        },
        size: Size2F {
            w: paper_size.get_drawable_rect().size.w,
            h: paper_size.get_drawable_rect().size.h - legend_area_rect.size.h
         }
    };

    let diamonds_painting_actual_rect = get_painting_actual_rect(
        &diamonds_painting_workspace_rect,
        &image_size,
        diamonds_step
    ).unwrap();

    println!("Paper drawable rect: {:?}", paper_size.get_drawable_rect());
    println!("Paining workspace rect: {diamonds_painting_workspace_rect:?}");
    println!("Legend rect: {legend_area_rect:?}, lines: {legend_lines_count}");
    println!("Paining actual rect: {diamonds_painting_actual_rect:?}");

    let mut document = Pdf::create(output_pdf_path)
        .expect("Create pdf file");

    // The 14 builtin fonts are available
    let font = BuiltinFont::Courier_Bold;

    // Add a page to the document.  This page will be 180 by 240 pt large.
    println!("{} x {}", paper_size.get_paper_size().w, paper_size.get_paper_size().h);
    document.render_page(
        paper_size.get_paper_size().w, 
        paper_size.get_paper_size().h, 
        |canvas| {
            
            let borders_line_width = 0.2;
            let diamonds_line_width = 0.1;
            // Printing margin
            {
                let (x, y, w, h) = paper_size.get_drawable_rect().into();
                println!("{x}, {y}, {w}, {h}");
                canvas.set_line_width(borders_line_width)?;
                canvas.set_stroke_color(Color::rgb(255, 0, 0)).unwrap();
                canvas.rectangle(x, y, w, h).unwrap();
                canvas.stroke().unwrap();
            }
            
            // Painting workspace
            {
                let (x, y, w, h) = diamonds_painting_workspace_rect.into();
                println!("{x}, {y}, {w}, {h}");
                canvas.set_line_width(borders_line_width)?;
                canvas.set_stroke_color(Color::rgb(255, 0, 255)).unwrap();
                canvas.rectangle(x, y, w, h).unwrap();
                canvas.stroke().unwrap();
            }
            
            // Painting rect
            {
                let (x, y, w, h) = diamonds_painting_actual_rect.into();
                println!("{x}, {y}, {w}, {h}");
                canvas.set_line_width(borders_line_width)?;
                canvas.set_stroke_color(Color::rgb(0, 255, 0)).unwrap();
                canvas.rectangle(x, y, w, h).unwrap();
                canvas.stroke().unwrap();
            }

            // Diamonds
            let rgb_img = img.to_rgb8();
            let mut diamonds_map: HashMap<Rgb<u8>, usize> = HashMap::new();
            rgb_img.enumerate_pixels()
                .for_each(|(_, _, pixel)| {
                    // count
                    diamonds_map.entry(*pixel).and_modify(|count| *count += 1).or_insert(1);
                });

            let diamonds_symbols_map: HashMap<Rgb<u8>, String> = diamonds_map.iter()
                .enumerate()
                .map(|(idx, (color, _))| {
                    (*color, char::from_u32(('A' as usize + idx) as u32).unwrap().to_string())
                })
                .collect();

            {
                let diamonds_origin = Pos2F {
                    x: diamonds_step / 2.0 + diamonds_painting_actual_rect.left(),
                    y: diamonds_step / 2.0 + diamonds_painting_actual_rect.bottom(),
                };
                canvas.set_line_width(diamonds_line_width)?;

                const DIAMOND_BG_WHITENESS: f32 = 0.0;
                rgb_img.enumerate_pixels()
                    .for_each(|(x, y, pixel)| {
                        // count
                        diamonds_map.entry(*pixel).and_modify(|count| *count += 1).or_insert(1);

                        // draw
                        let diamond_x = diamonds_origin.x + x as f32 * diamonds_step;
                        let diamond_y = diamonds_origin.y + y as f32 * diamonds_step;
                        let diamond_radius = diamond_diameter / 2.0;

                        let whiten_pixel = whiten_pixel(pixel, DIAMOND_BG_WHITENESS);
                        canvas.set_fill_color(Color::rgb(whiten_pixel.0[0], whiten_pixel.0[1], whiten_pixel.0[2])).unwrap();
                        canvas.rectangle(diamond_x - diamonds_step / 2.0, diamond_y - diamonds_step / 2.0, diamonds_step, diamonds_step).unwrap();
                        canvas.fill().unwrap();

                        canvas.circle(diamond_x, diamond_y, diamond_radius).unwrap();
                        canvas.set_fill_color(Color::rgb(255, 255, 255)).unwrap();
                        canvas.fill().unwrap();
                        
                        canvas.circle(diamond_x, diamond_y, diamond_radius).unwrap();
                        canvas.set_stroke_color(Color::rgb(0, 0, 0)).unwrap();
                        canvas.stroke().unwrap();

                        canvas.set_fill_color(Color::rgb(0, 0, 0)).unwrap();
                        canvas.center_text(
                            diamond_x, 
                            diamond_y - 0.55, 
                            font, 
                        2.0, 
                        diamonds_symbols_map.get(pixel).unwrap()
                        ).unwrap()
                    });
            }
            
            // Legend area
            {
                let (x, y, w, h) = legend_area_rect.into();
                println!("{x}, {y}, {w}, {h}");
                canvas.set_line_width(borders_line_width)?;
                canvas.set_stroke_color(Color::rgb(127, 127, 127)).unwrap();
                canvas.rectangle(x, y, w, h).unwrap();
                canvas.stroke().unwrap();
            }
            
            // Legend diamonds colors, counts, names
            {
                const LEGEND_COLOR_BAR_WIDTH: f32 = 18.0;
                const LEGEND_COLOR_BAR_GAP: f32 = 2.0;
                let legend_origin = legend_area_rect.pos;
                println!("diamonds_map={diamonds_map:?}");
                diamonds_map.iter().enumerate().for_each(|(line_num, (color, count))| {
                    canvas.set_fill_color(Color::rgb(color.0[0], color.0[1], color.0[2])).unwrap();
                    let line_y = legend_origin.y + LEGEND_LINE_MARGINS + line_num as f32 * LEGEND_LINE_HEIGHT;
                    canvas.rectangle(
                        legend_origin.x, 
                        line_y, 
                        LEGEND_COLOR_BAR_WIDTH,
                        LEGEND_LINE_HEIGHT - LEGEND_LINE_MARGINS * 2.0
                    ).unwrap();
                    canvas.fill().unwrap();

                    canvas.set_fill_color(Color::rgb(0, 0, 0)).unwrap();
                    let text = format!("[{}, {}, {}] × {}, symbol: {}", color.0[0], color.0[1], color.0[2], count, diamonds_symbols_map.get(color).unwrap());
                    canvas.left_text(
                        legend_origin.x + LEGEND_COLOR_BAR_WIDTH + LEGEND_COLOR_BAR_GAP, 
                        line_y + 0.79123, 
                        font, 
                        3.0, 
                        &text
                    ).unwrap()
                });
            }
            Ok(())
        }).expect("Write page");
    // Write all pending content, including the trailer and index
    document.finish().expect("Finish pdf document");
}

fn main()  {

    generate_project_pdf(
        "res/pink_8_colors_h_70.png", 
        "res/pink_8_colors.json", 
        PaperSize::VerticalA4, 
        2.0, 
        0.5, 
        "example.pdf"
    );

}