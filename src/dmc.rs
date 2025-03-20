use std::{
    collections::{HashMap, HashSet}, 
    io::BufReader, 
    ops::Deref
};

use ditherum::{
    color::ColorRGB, 
    palette::{errors::PaletteError, PaletteRGB}
};

use image::RgbImage;
use serde::{
    Deserialize, 
    Serialize
};

const PALETTE_PATH: &str = "res/palette_DMC.json";

#[derive(Debug, thiserror::Error)]
pub enum DmcError {
    #[error("Io error, reason: {0}")]
    IoError(#[from] std::io::Error),

    #[error("serde_json Io error, reason: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),

    #[error("Dmc data corrupted")]
    DmcDataCorrupted,

    #[error("Faled to parsecolor hex: {0}")]
    HexColorParseFailed(String),

    #[error("Faled to parse int in hex color: {0}")]
    HexColorParseIntFailed(#[from] std::num::ParseIntError),

    #[error("Data in DMC palette is not unique")]
    DmcDataNotUnique,

    #[error("PaletteDitherumError failed, reason={0}")]
    PaletteDitherumError(#[from] PaletteError),

    #[error("ColorNotFound")]
    ColorNotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DmcData {
    pub code: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaletteDmcData(pub Vec<DmcData>);


#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dmc {
    pub code: String,
    pub name: String,
    pub color: ColorRGB,
}

#[derive(Debug, Clone)]
pub struct ImageDmcLegendRecord {
    pub dmc: Dmc,
    pub count: usize,
    pub symbol: String
}

#[derive(Debug, Clone)]
pub struct ImageDmcLegend(pub HashMap<ColorRGB, ImageDmcLegendRecord>);

#[derive(Debug, Clone)]
pub struct PaletteDmc(pub Vec<Dmc>);

impl TryFrom<DmcData> for Dmc {
    type Error = DmcError;

    fn try_from(value: DmcData) -> Result<Self, Self::Error> {
        if value.code.is_empty() || value.color.is_empty() || value.name.is_empty() {
            return Err(Self::Error::DmcDataCorrupted);
        }

        let color = value.color;
        if !color.starts_with("#") || color.len() != 7 {
            return Err(Self::Error::HexColorParseFailed(color));
        }

        if !color[1..]
            .chars()
            .all(|c| c.is_ascii_hexdigit()) {
                return Err(Self::Error::HexColorParseFailed(color));
            }

        Ok(Dmc {
            code: value.code,
            name: value.name,
            color: ColorRGB([
                u8::from_str_radix(&color[1..3], 16)?,
                u8::from_str_radix(&color[3..5], 16)?,
                u8::from_str_radix(&color[5..], 16)?,
            ])
        })
    }
}

impl TryFrom<PaletteDmcData> for PaletteDmc {
    type Error = DmcError;

    fn try_from(value: PaletteDmcData) -> Result<Self, Self::Error> {
        // Must parse
        let dmc_vec: Result<Vec<Dmc>, Self::Error> = value.0.into_iter()
            .map(Dmc::try_from)
            .collect();
        let dmc_vec = dmc_vec?;

        // Must consist of unique names, codes and colors
        let unique_codes: HashSet<_> = dmc_vec.iter()
            .map(|dmc| dmc.code.clone())
            .collect();

        let unique_names: HashSet<_> = dmc_vec.iter()
            .map(|dmc| dmc.name.clone())
            .collect();

        let unique_colors: HashSet<_> = dmc_vec.iter()
            .map(|dmc| dmc.color)
            .collect();

        if unique_codes.len() != unique_names.len() || unique_codes.len() != unique_colors.len(){
            Err(Self::Error::DmcDataNotUnique)
        } else {
            Ok(Self(dmc_vec))
        }
    }
}

impl Deref for PaletteDmc {
    type Target = Vec<Dmc>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&PaletteDmc> for PaletteRGB {
    fn from(value: &PaletteDmc) -> Self {
        PaletteRGB::from(value.iter().map(|dmc| dmc.color).collect::<Vec<_>>())
    }
}

impl PaletteDmc {
    pub fn load_dmc_palette() -> Result<PaletteDmc, DmcError> {
        let file = std::fs::File::open(PALETTE_PATH)?;
        let file_reader = BufReader::new(file);
        let dmc_palette_data: PaletteDmcData = serde_json::from_reader(file_reader)?;
        let dmc_palette = PaletteDmc::try_from(dmc_palette_data)?;
        Ok(dmc_palette)
    }

    pub fn get_subset_closest_to(self, img_rgb: &RgbImage, colors_count: usize) -> Result<Self, DmcError> {
        // let mut result_dmc_vec: Vec<Dmc> = vec![];

        // rgb_palette.iter().for_each(|matched_color| {
        //     let closest_color = PaletteRGB::from(&self).find_closest_by_rgb(matched_color);

        //     // find line & move it out
        //     let color_index = self.0.iter().position(|dmc| dmc.color == closest_color).expect("Should be inside DMC Palette");
        //     let removed_dmc = self.0.remove(color_index);
        //     println!("closest:{:?} ~ {:?}", removed_dmc, matched_color);
        //     result_dmc_vec.push(removed_dmc);
        // });

        let rgb_palette = PaletteRGB::from(&self);
        let subset_palette = rgb_palette.clone().try_find_closest_subset_with_image(
            colors_count, 
            img_rgb, 
            true);

        let subset_palette = match subset_palette {
            Ok(palette) => palette,
            Err(e) => {
                if let PaletteError::RequestedTooManyColors { requested: _, possible } = e {
                    println!("RequestedTooManyColors: {e}");
                    rgb_palette.try_find_closest_subset_with_image(
                        possible, 
                        img_rgb, 
                        true)?
                } else {
                    return Err(e.into());
                }
            }
        };

        let result_dmc_vec: Option<Vec<Dmc>> = subset_palette.iter()
            .map(|color| {
                //find in DMC record 
                self.find_color_dmc(*color)
            })
            .collect();

        let result_dmc_vec = result_dmc_vec.ok_or(DmcError::ColorNotFound)?;

        Ok(Self(result_dmc_vec))
    }

    pub fn find_color_dmc(&self, color: ColorRGB) -> Option<Dmc> {
        let index = self.0.iter().position(|dmc| dmc.color == color)?;
        self.get(index).cloned()
    }
}

pub fn get_colors_counts(
    dithered_img: &RgbImage, 
) -> HashMap<ColorRGB, usize> {
    let mut colors_counts: HashMap<ColorRGB, usize> = HashMap::new();
    dithered_img.enumerate_pixels().for_each(|(_, _, px)| {
        let color_rgb = ColorRGB::from(*px);
        colors_counts.entry(color_rgb).and_modify(|count| *count += 1).or_insert(1);
    });
    colors_counts
}

impl ImageDmcLegend {
    pub fn extract_from(
        palette_dmc: &PaletteDmc, 
        colors_counts: &HashMap<ColorRGB, usize>,
        symbols: &[&str]
    ) -> Self {

        // // it can happen if image has less colors than DMC palette
        // if palette_dmc.len() != colors_histogram.len() {
        //     return Err(());
        // }

        let result_map: Option<HashMap<ColorRGB, ImageDmcLegendRecord>> = palette_dmc.iter()
            .enumerate()
            .map(|(idx, dmc)| {
                colors_counts.get(&dmc.color)
                    .map(|count| {
                        (
                            dmc.color, 
                            ImageDmcLegendRecord {
                                dmc: dmc.clone(),
                                count: *count,
                                symbol: symbols[idx].to_string()
                            }
                        )
                    })
                    
                })
            .collect();

        let result_map= result_map.unwrap(); //uhh do it better
        ImageDmcLegend(result_map)
    }
}

impl Deref for ImageDmcLegend {
    type Target = HashMap<ColorRGB, ImageDmcLegendRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[test]
fn test_loading_dmc_palette() {
    let palette = PaletteDmc::load_dmc_palette();
    assert!(palette.is_ok());

    let palette = palette.unwrap();
    assert!(!palette.is_empty());
}

#[test]
fn test_finding_closest_dmc_1_color_image() {
    let one_color_iamge = image::RgbImage::new(20, 20);
    let expected_colors_count = 1;

    let palette = PaletteDmc::load_dmc_palette().unwrap();

    let closest_palette: Result<PaletteDmc, DmcError> = palette.get_subset_closest_to(&one_color_iamge, expected_colors_count);
    assert!(closest_palette.is_ok());

    let closest_palette = closest_palette.unwrap();
    assert_eq!(expected_colors_count, closest_palette.len());
}

#[test]
fn test_finding_closest_dmc_not_enough_colors() {
    let one_color_iamge = image::RgbImage::new(20, 20);
    let requested_colors_count = 2;
    let expected_colors_count = 1;
    assert_ne!(expected_colors_count, requested_colors_count);

    let palette = PaletteDmc::load_dmc_palette().unwrap();

    let closest_palette: Result<PaletteDmc, DmcError> = palette.get_subset_closest_to(&one_color_iamge, requested_colors_count);
    assert!(closest_palette.is_ok());

    let closest_palette = closest_palette.unwrap();
    assert_eq!(expected_colors_count, closest_palette.len());
}