use font_kit::error::SelectionError;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;
use rustler::{Atom, Encoder, Env, OwnedBinary, Term};

// The OpenType specification requires every standalone sfnt to sum to this value.
const SFNT_CHECKSUM_MAGIC: u32 = 0xB1B0_AFBA;

mod atoms {
    rustler::atoms! {
        ok,
        error,
        not_found,
        unavailable,
        unsupported_font,
        normal,
        italic,
        oblique
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolveError {
    NotFound,
    Unavailable,
    UnsupportedFont,
}

#[rustler::nif(schedule = "DirtyIo")]
fn resolve(env: Env<'_>, family: String, weight: f32, style: Atom) -> Term<'_> {
    match resolve_font(family, weight, style) {
        Ok((resolved_family, resolved_weight, resolved_style, data)) => {
            let Some(mut binary) = OwnedBinary::new(data.len()) else {
                return (atoms::error(), atoms::unavailable()).encode(env);
            };

            binary.as_mut_slice().copy_from_slice(&data);

            (
                atoms::ok(),
                resolved_family,
                resolved_weight,
                resolved_style,
                binary.release(env),
            )
                .encode(env)
        }
        Err(ResolveError::NotFound) => (atoms::error(), atoms::not_found()).encode(env),
        Err(ResolveError::Unavailable) => (atoms::error(), atoms::unavailable()).encode(env),
        Err(ResolveError::UnsupportedFont) => {
            (atoms::error(), atoms::unsupported_font()).encode(env)
        }
    }
}

fn resolve_font(
    family: String,
    weight: f32,
    style: Atom,
) -> Result<(String, f32, Atom, Vec<u8>), ResolveError> {
    let requested_style = if style == atoms::italic() {
        Style::Italic
    } else if style == atoms::oblique() {
        Style::Oblique
    } else {
        Style::Normal
    };

    let mut properties = Properties::new();
    properties.weight(Weight(weight)).style(requested_style);

    let family_names = requested_family_names(family);

    let source = SystemSource::new();
    let mut found_unavailable_font = false;
    let mut found_unsupported_font = false;

    for family_name in family_names {
        let handle = match source.select_best_match(&[family_name], &properties) {
            Ok(handle) => handle,
            Err(error) => match classify_selection_error(error) {
                ResolveError::NotFound => continue,
                error => return Err(error),
            },
        };
        let font = match handle.load() {
            Ok(font) => font,
            Err(_) => {
                found_unavailable_font = true;
                continue;
            }
        };
        let data = match standalone_font_data(&handle) {
            Ok(data) => data,
            Err(ResolveError::Unavailable) => {
                found_unavailable_font = true;
                continue;
            }
            Err(ResolveError::UnsupportedFont) => {
                found_unsupported_font = true;
                continue;
            }
            Err(ResolveError::NotFound) => continue,
        };
        let actual_properties = font.properties();
        let actual_style = match actual_properties.style {
            Style::Italic => atoms::italic(),
            Style::Oblique => atoms::oblique(),
            Style::Normal => atoms::normal(),
        };

        return Ok((
            font.family_name(),
            actual_properties.weight.0,
            actual_style,
            data,
        ));
    }

    if found_unavailable_font {
        Err(ResolveError::Unavailable)
    } else if found_unsupported_font {
        Err(ResolveError::UnsupportedFont)
    } else {
        Err(ResolveError::NotFound)
    }
}

fn classify_selection_error(error: SelectionError) -> ResolveError {
    match error {
        SelectionError::NotFound => ResolveError::NotFound,
        SelectionError::CannotAccessSource { .. } => ResolveError::Unavailable,
    }
}

fn requested_family_names(family: String) -> Vec<FamilyName> {
    match family.to_ascii_lowercase().as_str() {
        "serif" => vec![FamilyName::Serif],
        "sans-serif" => vec![FamilyName::SansSerif],
        "system-ui" => system_ui_family_names(),
        "monospace" => vec![FamilyName::Monospace],
        _ => vec![FamilyName::Title(family)],
    }
}

#[cfg(target_os = "linux")]
fn system_ui_family_names() -> Vec<FamilyName> {
    vec![
        FamilyName::Title("Cantarell".to_owned()),
        FamilyName::Title("Noto Sans UI".to_owned()),
        FamilyName::Title("Segoe UI".to_owned()),
        FamilyName::SansSerif,
    ]
}

#[cfg(target_os = "macos")]
fn system_ui_family_names() -> Vec<FamilyName> {
    vec![
        FamilyName::Title(".AppleSystemUIFont".to_owned()),
        FamilyName::Title("SF Pro".to_owned()),
        FamilyName::Title("Helvetica Neue".to_owned()),
        FamilyName::SansSerif,
    ]
}

#[cfg(target_family = "windows")]
fn system_ui_family_names() -> Vec<FamilyName> {
    vec![
        FamilyName::Title("Segoe UI".to_owned()),
        FamilyName::SansSerif,
    ]
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_family = "windows")))]
fn system_ui_family_names() -> Vec<FamilyName> {
    vec![FamilyName::SansSerif]
}

fn standalone_font_data(handle: &Handle) -> Result<Vec<u8>, ResolveError> {
    let data = match handle {
        Handle::Path { path, font_index } => {
            let data = std::fs::read(path).map_err(|_| ResolveError::Unavailable)?;
            extract_collection_face(&data, *font_index as usize)
                .map_err(|_| ResolveError::Unavailable)?
        }
        Handle::Memory { bytes, font_index } => {
            extract_collection_face(bytes.as_ref(), *font_index as usize)
                .map_err(|_| ResolveError::Unavailable)?
        }
    };

    match sfnt_has_table(&data, b"fvar") {
        Some(true) => Err(ResolveError::UnsupportedFont),
        Some(false) => Ok(data),
        None => Err(ResolveError::Unavailable),
    }
}

fn extract_collection_face(data: &[u8], font_index: usize) -> Result<Vec<u8>, ()> {
    if data.get(0..4) != Some(b"ttcf".as_slice()) {
        return supported_sfnt(data).then(|| data.to_vec()).ok_or(());
    }

    let count = read_u32(data, 8).ok_or(())? as usize;
    if font_index >= count {
        return Err(());
    }

    let face_offset = read_u32(data, 12 + font_index * 4).ok_or(())? as usize;
    let scaler = data.get(face_offset..face_offset + 4).ok_or(())?;
    if scaler != [0, 1, 0, 0] && scaler != b"true" && scaler != b"typ1" && scaler != b"OTTO" {
        return Err(());
    }

    let table_count = read_u16(data, face_offset + 4).ok_or(())? as usize;
    let directory_end = face_offset + 12 + table_count * 16;
    if directory_end > data.len() {
        return Err(());
    }

    let mut output = data[face_offset..directory_end].to_vec();
    let mut output_offset = 12 + table_count * 16;
    let mut head_table = None;

    for index in 0..table_count {
        let record = face_offset + 12 + index * 16;
        let tag = data.get(record..record + 4).ok_or(())?;
        let source_offset = read_u32(data, record + 8).ok_or(())? as usize;
        let length = read_u32(data, record + 12).ok_or(())? as usize;
        let source_end = source_offset.checked_add(length).ok_or(())?;
        let table = data.get(source_offset..source_end).ok_or(())?;

        while output.len() < output_offset {
            output.push(0);
        }

        output.extend_from_slice(table);
        let new_offset = output_offset as u32;
        output[12 + index * 16 + 8..12 + index * 16 + 12]
            .copy_from_slice(&new_offset.to_be_bytes());

        if tag == b"head" {
            head_table = Some((output_offset, length));
        }

        output_offset += length;
        output_offset = (output_offset + 3) & !3;
    }

    while output.len() < output_offset {
        output.push(0);
    }

    let (head_offset, head_length) = head_table.ok_or(())?;
    set_checksum_adjustment(&mut output, head_offset, head_length)?;

    Ok(output)
}

fn set_checksum_adjustment(
    font: &mut [u8],
    head_offset: usize,
    head_length: usize,
) -> Result<(), ()> {
    if head_length < 12 {
        return Err(());
    }

    let adjustment_offset = head_offset.checked_add(8).ok_or(())?;
    let adjustment_end = adjustment_offset.checked_add(4).ok_or(())?;
    font.get_mut(adjustment_offset..adjustment_end)
        .ok_or(())?
        .fill(0);

    let adjustment = SFNT_CHECKSUM_MAGIC.wrapping_sub(sfnt_checksum(font));
    font[adjustment_offset..adjustment_end].copy_from_slice(&adjustment.to_be_bytes());

    Ok(())
}

fn sfnt_checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0, |sum, bytes| {
        let mut word = [0; 4];
        word[..bytes.len()].copy_from_slice(bytes);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn supported_sfnt(data: &[u8]) -> bool {
    matches!(
        data.get(0..4),
        Some([0, 1, 0, 0]) | Some(b"true") | Some(b"typ1") | Some(b"OTTO")
    )
}

fn sfnt_has_table(data: &[u8], wanted_tag: &[u8; 4]) -> Option<bool> {
    let table_count = read_u16(data, 4)? as usize;
    let directory_size = table_count.checked_mul(16)?;
    let directory_end = 12_usize.checked_add(directory_size)?;
    let directory = data.get(12..directory_end)?;

    Some(
        directory
            .chunks_exact(16)
            .any(|record| record.get(0..4) == Some(wanted_tag.as_slice())),
    )
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    Some(u16::from_be_bytes(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(bytes))
}

rustler::init!("Elixir.ElixirFontDiscovery.Native");

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use font_kit::handle::Handle;

    use super::{
        classify_selection_error, extract_collection_face, requested_family_names, sfnt_checksum,
        sfnt_has_table, standalone_font_data, supported_sfnt, FamilyName, ResolveError,
        SelectionError, SFNT_CHECKSUM_MAGIC,
    };

    #[test]
    fn accepts_standalone_true_type_data() {
        let data = [0, 1, 0, 0, 0, 0, 0, 0];

        assert!(supported_sfnt(&data));
        assert_eq!(extract_collection_face(&data, 0), Ok(data.to_vec()));
    }

    #[test]
    fn rejects_unsupported_or_truncated_data() {
        assert!(!supported_sfnt(b"WOFF"));
        assert_eq!(extract_collection_face(b"WOFF", 0), Err(()));
        assert_eq!(extract_collection_face(b"ttcf", 0), Err(()));

        assert_eq!(
            standalone_font_data(&Handle::Memory {
                bytes: Arc::new(b"WOFF".to_vec()),
                font_index: 0,
            }),
            Err(ResolveError::Unavailable)
        );
    }

    #[test]
    fn reports_file_and_extraction_failures_as_unavailable() {
        let missing_path = std::env::temp_dir().join(format!(
            "elixir-font-discovery-missing-{}",
            std::process::id()
        ));
        assert!(!missing_path.exists());
        assert_eq!(
            standalone_font_data(&Handle::Path {
                path: missing_path,
                font_index: 0,
            }),
            Err(ResolveError::Unavailable)
        );

        let mut bad_index = vec![0; 16];
        bad_index[0..4].copy_from_slice(b"ttcf");
        bad_index[8..12].copy_from_slice(&1_u32.to_be_bytes());
        assert_eq!(
            standalone_font_data(&Handle::Memory {
                bytes: Arc::new(bad_index),
                font_index: 1,
            }),
            Err(ResolveError::Unavailable)
        );

        assert_eq!(
            standalone_font_data(&Handle::Memory {
                bytes: Arc::new(vec![0, 1, 0, 0, 0, 1]),
                font_index: 0,
            }),
            Err(ResolveError::Unavailable)
        );
    }

    #[test]
    fn accepts_cff_open_type_data() {
        assert!(supported_sfnt(b"OTTOfont"));
        assert_eq!(
            extract_collection_face(b"OTTOfont", 0),
            Ok(b"OTTOfont".to_vec())
        );
    }

    #[test]
    fn maps_system_ui_separately_from_sans_serif() {
        let system_ui = requested_family_names("system-ui".to_owned());
        let sans_serif = requested_family_names("sans-serif".to_owned());

        assert_ne!(system_ui, sans_serif);
        assert_eq!(system_ui.last(), Some(&FamilyName::SansSerif));

        #[cfg(target_os = "linux")]
        assert_eq!(
            system_ui.first(),
            Some(&FamilyName::Title("Cantarell".to_owned()))
        );

        #[cfg(target_os = "macos")]
        assert_eq!(
            system_ui.first(),
            Some(&FamilyName::Title(".AppleSystemUIFont".to_owned()))
        );

        #[cfg(target_family = "windows")]
        assert_eq!(
            system_ui.first(),
            Some(&FamilyName::Title("Segoe UI".to_owned()))
        );
    }

    #[test]
    fn distinguishes_selection_misses_from_source_failures() {
        assert_eq!(
            classify_selection_error(SelectionError::NotFound),
            ResolveError::NotFound
        );
        assert_eq!(
            classify_selection_error(SelectionError::CannotAccessSource { reason: None }),
            ResolveError::Unavailable
        );
    }

    #[test]
    fn identifies_variable_font_tables() {
        let mut font = vec![0; 44];
        font[0..4].copy_from_slice(&[0, 1, 0, 0]);
        font[4..6].copy_from_slice(&2_u16.to_be_bytes());
        font[12..16].copy_from_slice(b"fvar");
        font[28..32].copy_from_slice(b"name");

        assert_eq!(sfnt_has_table(&font, b"fvar"), Some(true));
        assert_eq!(
            standalone_font_data(&Handle::Memory {
                bytes: Arc::new(font.clone()),
                font_index: 0,
            }),
            Err(ResolveError::UnsupportedFont)
        );
        assert_eq!(sfnt_has_table(&font, b"gvar"), Some(false));
        assert_eq!(sfnt_has_table(&font[..20], b"fvar"), None);
    }

    #[test]
    fn extracts_a_standalone_face_from_a_collection() {
        let mut collection = vec![0; 124];
        collection[0..4].copy_from_slice(b"ttcf");
        collection[8..12].copy_from_slice(&1_u32.to_be_bytes());
        collection[12..16].copy_from_slice(&16_u32.to_be_bytes());
        collection[16..20].copy_from_slice(&[0, 1, 0, 0]);
        collection[20..22].copy_from_slice(&2_u16.to_be_bytes());
        collection[28..32].copy_from_slice(b"head");
        collection[36..40].copy_from_slice(&64_u32.to_be_bytes());
        collection[40..44].copy_from_slice(&54_u32.to_be_bytes());
        collection[44..48].copy_from_slice(b"name");
        collection[52..56].copy_from_slice(&120_u32.to_be_bytes());
        collection[56..60].copy_from_slice(&4_u32.to_be_bytes());
        collection[64..68].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
        collection[72..76].copy_from_slice(&0x1234_5678_u32.to_be_bytes());
        collection[76..80].copy_from_slice(&0x5F0F_3CF5_u32.to_be_bytes());
        collection[82..84].copy_from_slice(&1_000_u16.to_be_bytes());
        collection[120..124].copy_from_slice(b"font");

        let extracted = extract_collection_face(&collection, 0).expect("valid collection face");

        assert_eq!(&extracted[0..4], &[0, 1, 0, 0]);
        assert_eq!(&extracted[20..24], &(44_u32.to_be_bytes()));
        assert_eq!(&extracted[36..40], &(100_u32.to_be_bytes()));
        assert_eq!(&extracted[100..104], b"font");
        assert_ne!(&extracted[52..56], &0x1234_5678_u32.to_be_bytes());
        assert_eq!(sfnt_checksum(&extracted), SFNT_CHECKSUM_MAGIC);
    }
}
