use ditherum::color::ColorRGB;
use image::{
    Rgb, 
    RgbImage
};
use millimeter::mm;
use pdf_canvas::{
    graphicsstate::Color, 
    BuiltinFont, 
    Canvas, 
    Pdf
};
use crate::{
    dmc::ImageDmcLegend, 
    types::{
        DiamondShape, 
        PaperSheet, 
        Pos2D, 
        Rect2D, 
        Size2D
    }
};

/// Points -> mm
/// 3.0    -> 1.0583
/// 1.0    -> 0.3528
/// 0.25   -> 0.0882
fn mm_to_points(value: mm) -> f32 {
    72.0 * value.raw_value() / 25.4
}

fn rect_to_points(rect: &Rect2D) -> (f32, f32, f32, f32) {
    (
        mm_to_points(rect.pos.x),
        mm_to_points(rect.pos.y),
        mm_to_points(rect.size.w),
        mm_to_points(rect.size.h)
    )
}

fn draw_filled_rect(
    canvas: &mut Canvas,
    rect: &Rect2D,
    filling_color: Color
) -> std::io::Result<()> {
    let (x, y, w, h) = rect_to_points(rect);
    canvas.set_fill_color(filling_color)?;
    canvas.rectangle(x, y, w, h)?;
    canvas.fill()
}

fn draw_empty_bordered_rect(
    canvas: &mut Canvas,
    rect: &Rect2D,
    line_thickness_pt: f32,
    border_color: Color
) -> std::io::Result<()> {
    let (x, y, w, h) = rect_to_points(rect);
    canvas.set_line_width(line_thickness_pt)?;
    canvas.set_stroke_color(border_color)?;
    canvas.rectangle(x, y, w, h)?;
    canvas.stroke()
}

pub fn render_diamond_painting_project(
    paper_sheet: PaperSheet,
    diamond_shape: DiamondShape,
    dmc_image_legend: ImageDmcLegend,
    dithered_img: RgbImage,
    draw_template_lines: bool,
    output_path: &str,
) -> std::io::Result<()> {
    const TEMPLATE_LINES_THICKNESS_PT: f32 = 0.75;

    let mut document = Pdf::create(output_path)
        .expect("Create pdf file");

    // Use builtin font
    let font = BuiltinFont::Courier_Bold;

    let printing_area_rect = paper_sheet.get_printing_area_rect();
    let img_size = Size2D {
        w: dithered_img.width() as f32 * diamond_shape.get_size(),
        h: dithered_img.height() as f32 * diamond_shape.get_size(),
    };
    let image_occupied_area_rect = printing_area_rect.get_centered(&img_size);

    // Painting image
    document.render_page(
        mm_to_points(paper_sheet.size.w),
        mm_to_points(paper_sheet.size.h),
        |canvas| {
            
            if draw_template_lines {
                // Margins
                draw_empty_bordered_rect(
                    canvas, 
                    &printing_area_rect, 
                    TEMPLATE_LINES_THICKNESS_PT,
                    Color::rgb(255, 0, 0)
                )?;

                // Occupied area
                draw_empty_bordered_rect(
                    canvas, 
                    &image_occupied_area_rect, 
                    TEMPLATE_LINES_THICKNESS_PT,
                    Color::rgb(0, 255, 0)
                )?;
            }

            // Diamonds
            let flip_y = dithered_img.height();
            let symbol_font_size = mm_to_points(mm::new(2.2));
            let symbol_x_oiffset = mm_to_points(diamond_shape.get_size()) / 2.0;
            let symbol_y_oiffset = mm_to_points(diamond_shape.get_size()) / 4.0;

            dithered_img.enumerate_pixels()
                .try_for_each(|(x, y, pixel)| {
                    let pixel_rect = Rect2D {
                        pos: Pos2D {
                            x: image_occupied_area_rect.pos.x + x as f32 * diamond_shape.get_size(),
                            y: image_occupied_area_rect.pos.y + (flip_y - y - 1) as f32 * diamond_shape.get_size(),
                        },
                        size: Size2D::new_square(diamond_shape.get_size())
                    };
                    
                    // Pixel's background
                    draw_filled_rect(
                        canvas, 
                        &pixel_rect, 
                        Color::rgb(pixel.0[0], pixel.0[1], pixel.0[2])
                    )?;

                    // Symbol
                    let symbol = dmc_image_legend.get(&ColorRGB::from(*pixel))
                        .map(|ldmc| ldmc.symbol.to_string())
                        .unwrap_or(String::from('!'));

                    // Draw contrasting color
                    canvas.set_fill_color(get_contrasting_color(pixel))?;
                    canvas.center_text(
                        mm_to_points(pixel_rect.pos.x) + symbol_x_oiffset, 
                        mm_to_points(pixel_rect.pos.y) + symbol_y_oiffset, 
                        font, 
                        symbol_font_size, 
                        &symbol
                    )?;

                    Ok(())
                })
        })?;

    // Write all pending content, including the trailer and index
    document.finish()
}

fn get_contrasting_color(pixel: &Rgb<u8>) -> Color {
    let channel_color = pixel.0[0] as u32 + pixel.0[1] as u32 + pixel.0[2] as u32;
    let channel_color = if channel_color > 300 { 0 } else { 255 };
    Color::rgb(channel_color, channel_color, channel_color)
}