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

pub fn extract_palette_subset<P: AsRef<Path>> (
    paper_sheet: PaperSheet,
    provided_dmc_palette: PaletteDmc,
    max_colors_count: usize,
    diamond_shape: DiamondShape,
    image_path: P
) -> Result<PaletteDmc, ProcessError> {
    let max_colors_count = max_colors_count.min(PALLETE_LEN_MAX);

    // Fit image to printable area
    let img_rgb = image::open(image_path)?
        .to_rgb8();

    let (_, img_rgb) = fit_image_on_paper_printable_area(paper_sheet, &diamond_shape, img_rgb);
    
    let dmc_subset_palette = provided_dmc_palette.get_subset_closest_to(&img_rgb, max_colors_count)?;
    Ok(dmc_subset_palette)
}

pub fn process_image_with_path<P: AsRef<Path>> (
    paper_sheet: PaperSheet,
    provided_dmc_palette: PaletteDmc,
    max_colors_count: usize,
    diamond_shape: DiamondShape,
    image_path: P,
    preview_path: Option<P>,
    dmc_palette_path: Option<P>,
    output_path: &str,
) -> Result<PaletteDmc, ProcessError> {
    let max_colors_count = max_colors_count.min(PALLETE_LEN_MAX);

    // Fit image to printable area
    let img_rgb = image::open(image_path)?
        .to_rgb8();
    let (paper_sheet, img_rgb) = fit_image_on_paper_printable_area(
        paper_sheet, 
        &diamond_shape, 
        img_rgb
    );
    
    let dmc_subset_palette = provided_dmc_palette.get_subset_closest_to(&img_rgb, max_colors_count)?;

    let dithered_img = dithering_floyd_steinberg_rgb(
        img_rgb, 
        PaletteRGB::from(&dmc_subset_palette)
    );
    
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

    Ok(dmc_subset_palette)
}

#[cfg(test)]
mod test_generator {
    use std::path::Path;

    use crate::{
        dmc::PaletteDmc, 
        generator::extract_palette_subset, 
        types::{
            DiamondShape, 
            PaperSheet
        }
    };
    use super::{
        process_image_with_path, 
        ProcessError
    };

    fn full_generate_helper(
        paper_sheet: PaperSheet,
        provided_dmc_palette: PaletteDmc,
        image_filename: &str,
        max_colors_count: usize
    ) -> Result<PaletteDmc, ProcessError> {
        let filename_stem = Path::new(image_filename)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap();

        process_image_with_path(
            paper_sheet,
            provided_dmc_palette,
            max_colors_count,
            DiamondShape::common_round(),
            format!("res/{image_filename}").as_str(),
            Some(format!("res/outputs/{filename_stem}_preview.png").as_str()),
            Some(format!("res/outputs/{filename_stem}_dmc_palette.json").as_str()),
            format!("res/outputs/{filename_stem}.pdf").as_str(),
        )
    }

    #[test]
    fn test_process_image_with_path_a4_max_12_colors() {
        let max_colors_count = 12;
        let processing_result = full_generate_helper(
            PaperSheet::standard_a4(),
            PaletteDmc::load_dmc_palette().unwrap(),
            "test_pink_300.jpg",
            max_colors_count
        );
    
        assert!(processing_result.is_ok());
        let processing_result = processing_result.unwrap();

        assert!(processing_result.len() <= max_colors_count);
    }

    #[test]
    fn test_process_image_with_path_a3_max_32_colors() {
        let max_colors_count = 32;
        let processing_result = full_generate_helper(
            PaperSheet::standard_a3(),
            PaletteDmc::load_dmc_palette().unwrap(),
            "test_yellow_600.jpg",
            max_colors_count
        );
    
        assert!(processing_result.is_ok());
        let processing_result = processing_result.unwrap();

        assert!(processing_result.len() <= max_colors_count);
    }
    
    #[test]
    fn test_find_subset_palette() {
        let provided_dmc_palette = PaletteDmc::load_dmc_palette().unwrap();
        let max_colors_count = 12;
        
        let processing_result = extract_palette_subset(
            PaperSheet::standard_a4(),
            provided_dmc_palette,
            max_colors_count,
            DiamondShape::common_round(),
            "res/test_pink_300.jpg"
        );
        assert!(processing_result.is_ok());
    
        let processing_result = processing_result.unwrap();
        assert!(processing_result.len() <= max_colors_count);
    }
}
