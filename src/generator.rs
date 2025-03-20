use std::path::Path;

use ditherum::{
    algorithms::dithering::dithering_floyd_steinberg_rgb, 
    image::manip::rgb_image_reshape, 
    palette::{errors::PaletteError, PaletteRGB}
};

use image::{ImageError, RgbImage};

use crate::{
    dmc::{
        get_colors_counts, DmcError, ImageDmcLegend, PaletteDmc
    }, 
    render::render_diamond_painting_project, 
    types::{
        DiamondShape, 
        PaperSheet, 
        Size2F, 
        Size2U
    }
};

const LABEL_SYMBOLS: [&str; 32] = [
    "1", "2", "4", "5", "7", "9",
    "A", "B", "C", "W", "X", "S", "R",
    "a", "i", "m", "h", "r",
    "c", "u", "z", "q", "Q", "8", "Y", "+", "=", "@", "#", "$", "%", "*"
];
// "★", "✦", "❖", "⌖", "▲", "⬠", "⊙", "☾", "⌘", "✪", "♡", "♞", "♠", "♛",

const PALLETE_LEN_MAX: usize = LABEL_SYMBOLS.len();

#[derive(Debug, Clone)]
pub struct PreprocessResult {
    pub paper_sheet: PaperSheet,
    pub image_size: Size2U,
    pub palette: PaletteRGB,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Open image failed, reason={0}")]
    ImageError(#[from] ImageError),

    #[error("LoadDmcPaletteError, reason={0}")]
    LoadDmcPaletteError(#[from] DmcError),

    #[error("PaletteError, reason={0}")]
    PaletteError(#[from] PaletteError),

    #[error("IoError, reason={0}")]
    IoError(#[from] std::io::Error),

    #[error("BadColorsCount: expected={expected}, possible={possible}")]
    BadColorsCount {
        expected: usize,
        possible: usize
    },
}

fn fit_image_on_paper_printable_area(mut paper_sheet: PaperSheet, diamond_shape: &DiamondShape, rgb_img: RgbImage) -> (PaperSheet, RgbImage) {
    let rgb_img_is_vertical = Size2F {
        w: rgb_img.width() as f32,
        h: rgb_img.height() as f32
    }.is_vertical();
    let printable_area_is_vertical = Size2F::from(&paper_sheet.size).is_vertical();

    if rgb_img_is_vertical != printable_area_is_vertical {
        paper_sheet.change_orientation();
    }

    let expected_width_in_pixels = (paper_sheet.get_printing_area_rect().size.w / diamond_shape.get_size()).round() as u32;
    let result_img = rgb_image_reshape(
        rgb_img, 
        Some(expected_width_in_pixels), 
        None
    );

    (paper_sheet, result_img)
}


pub fn process_image_with_path<P: AsRef<Path>> (
    paper_sheet: PaperSheet,
    colors_count: usize,
    diamond_shape: DiamondShape,
    image_path: P,
    preview_path: Option<P>,
    dmc_palette_path: Option<P>,
    output_path: &str,
) -> Result<(), ProcessError> {
    if colors_count > PALLETE_LEN_MAX {
        unimplemented!("todo: add error colors_count > PALLETE_LEN_MAX");
    }
    // Fit image to printable area
    let img_rgb = image::open(image_path)?
        .to_rgb8();
    let (paper_sheet, img_rgb) = fit_image_on_paper_printable_area(paper_sheet, &diamond_shape, img_rgb);
    
    let dmc_full_palette = PaletteDmc::load_dmc_palette()?;
    let dmc_subset_palette = dmc_full_palette.get_subset_closest_to(&img_rgb, colors_count)?;

    let dithered_img = dithering_floyd_steinberg_rgb(img_rgb, PaletteRGB::from(&dmc_subset_palette));
    if let Some(path) = preview_path {
        dithered_img.save(path)?;
    }

    let colors_counts = get_colors_counts(&dithered_img);

    if let Some(_path) = dmc_palette_path {
        println!("Todo use path for palette");
    }

    if dmc_subset_palette.len() != colors_counts.len() {
        return Err(ProcessError::BadColorsCount {expected: dmc_subset_palette.len(), possible: colors_counts.len()})
    }

    let dmc_image_legend = ImageDmcLegend::extract_from(
        &dmc_subset_palette, 
        &colors_counts, 
        &LABEL_SYMBOLS
    );
    // println!("{dmc_image_legend:?}");

    render_diamond_painting_project(
        paper_sheet,
        diamond_shape,
        dmc_image_legend,
        dithered_img,
        true,
        output_path
    )?;

    Ok(())
}

#[test]
fn test_process_image_with_path_a4_12_colors() {
    let colors_count = 12;
    
    let processing_result = process_image_with_path(
        PaperSheet::standard_a4(),
        colors_count,
        DiamondShape::common_round(),
        "res/test_pink_300.jpg",
        Some("res/outputs/test_pink_300_preview.png"),
        Some("res/outputs/test_pink_300_dmc_palette.json"),
        "res/outputs/test_pink.pdf",
    );

    assert!(processing_result.is_ok())
}

#[test]
fn test_process_image_with_path_a3_16_colors() {
    let colors_count = 16;
    
    let processing_result = process_image_with_path(
        PaperSheet::standard_a3(),
        colors_count,
        DiamondShape::common_round(),
        "res/test_pink_300.jpg",
        Some("res/outputs/test_pink_300_preview_a3.png"),
        Some("res/outputs/test_pink_300_dmc_palette_a3.json"),
        "res/outputs/test_pink_a3.pdf",
    );

    assert!(processing_result.is_ok())
}

#[test]
fn test_process_image_with_path_testyard() {
    let colors_count = 21; // 22 seems too many
    let src_name = "test_grass_300.png";
    
    let processing_result = process_image_with_path(
        PaperSheet::standard_a4(),
        colors_count,
        DiamondShape::common_round(),
        format!("res/{src_name}").as_str(),
        Some(format!("res/outputs/{src_name}_preview.png").as_str()),
        Some(format!("res/outputs/{src_name}_dmc_palette_a3.json").as_str()),
        format!("res/outputs/{src_name}_print.pdf").as_str()
    );

    assert!(processing_result.is_ok(), "meh {processing_result:?}")
}

