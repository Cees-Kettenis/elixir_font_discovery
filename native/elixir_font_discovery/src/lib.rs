use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::{Properties, Style, Weight};
use font_kit::source::SystemSource;
use rustler::{Atom, Encoder, Env, OwnedBinary, Term};

mod atoms {
    rustler::atoms! {
        ok,
        error,
        not_found,
        unavailable,
        normal,
        italic,
        oblique
    }
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
        Err(()) => (atoms::error(), atoms::not_found()).encode(env),
    }
}

fn resolve_font(
    family: String,
    weight: f32,
    style: Atom,
) -> Result<(String, f32, Atom, Vec<u8>), ()> {
    let requested_style = if style == atoms::italic() {
        Style::Italic
    } else if style == atoms::oblique() {
        Style::Oblique
    } else {
        Style::Normal
    };

    let mut properties = Properties::new();
    properties.weight(Weight(weight)).style(requested_style);

    let family_name = match family.to_ascii_lowercase().as_str() {
        "serif" => FamilyName::Serif,
        "sans-serif" | "system-ui" => FamilyName::SansSerif,
        "monospace" => FamilyName::Monospace,
        _ => FamilyName::Title(family),
    };

    let handle = SystemSource::new()
        .select_best_match(&[family_name], &properties)
        .map_err(|_| ())?;
    let font = handle.load().map_err(|_| ())?;
    let data = standalone_font_data(&handle)?;
    let actual_properties = font.properties();
    let actual_style = match actual_properties.style {
        Style::Italic => atoms::italic(),
        Style::Oblique => atoms::oblique(),
        Style::Normal => atoms::normal(),
    };

    Ok((
        font.family_name(),
        actual_properties.weight.0,
        actual_style,
        data,
    ))
}

fn standalone_font_data(handle: &Handle) -> Result<Vec<u8>, ()> {
    match handle {
        Handle::Path { path, font_index } => {
            let data = std::fs::read(path).map_err(|_| ())?;
            extract_collection_face(&data, *font_index as usize)
        }
        Handle::Memory { bytes, font_index } => {
            extract_collection_face(bytes.as_ref(), *font_index as usize)
        }
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

    for index in 0..table_count {
        let record = face_offset + 12 + index * 16;
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
        output_offset += length;
        output_offset = (output_offset + 3) & !3;
    }

    while output.len() < output_offset {
        output.push(0);
    }

    Ok(output)
}

fn supported_sfnt(data: &[u8]) -> bool {
    matches!(
        data.get(0..4),
        Some([0, 1, 0, 0]) | Some(b"true") | Some(b"typ1") | Some(b"OTTO")
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
    use super::{extract_collection_face, supported_sfnt};

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
    fn extracts_a_standalone_face_from_a_collection() {
        let mut collection = vec![0; 60];
        collection[0..4].copy_from_slice(b"ttcf");
        collection[8..12].copy_from_slice(&1_u32.to_be_bytes());
        collection[12..16].copy_from_slice(&16_u32.to_be_bytes());
        collection[16..20].copy_from_slice(&[0, 1, 0, 0]);
        collection[20..22].copy_from_slice(&1_u16.to_be_bytes());
        collection[28..32].copy_from_slice(b"name");
        collection[36..40].copy_from_slice(&48_u32.to_be_bytes());
        collection[40..44].copy_from_slice(&4_u32.to_be_bytes());
        collection[48..52].copy_from_slice(b"font");

        let extracted = extract_collection_face(&collection, 0).expect("valid collection face");

        assert_eq!(&extracted[0..4], &[0, 1, 0, 0]);
        assert_eq!(&extracted[20..24], &(28_u32.to_be_bytes()));
        assert_eq!(&extracted[28..32], b"font");
    }
}
