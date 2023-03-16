use std::fs::OpenOptions;
use std::io::{Read, Write};

use anyhow::{anyhow, Result};
use encoding::{DecoderTrap, EncoderTrap};
use tempfile::NamedTempFile;

use crate::encoding::{get_encoder, Bom};
use crate::model::ReplacementCriteria;
use crate::rg::de::{ArbitraryData, SubMatch};
use crate::rg::RgEncoding;
use crate::ui::line::Item;

fn perform_replacements_in_file(
    criteria: &ReplacementCriteria,
    rg_encoding: &RgEncoding,
    (path_data, mut items): (&ArbitraryData, Vec<&Item>),
) -> Result<bool> {
    log::debug!("File: {} (item count: {})", path_data, items.len());
    let path_buf = path_data.to_path_buf()?;

    // Check the file for a BOM, detect its encoding and then decode it into a string.
    let (bom, encoder, mut file_as_str) = {
        let mut file_contents = vec![];
        OpenOptions::new()
            .read(true)
            .open(&path_buf)?
            .read_to_end(&mut file_contents)?;

        // Search for a BOM and attempt to detect file encoding.
        let (bom, encoder) = get_encoder(&file_contents, rg_encoding);
        log::debug!("BOM: {:?}", bom);
        log::debug!("Encoder: {}", encoder.name());

        // Strip the BOM before we decode.
        match bom {
            // NOTE: we don't strip a UTF8 BOM, because ripgrep doesn't either
            // See: https://github.com/BurntSushi/ripgrep/issues/1638
            None | Some(Bom::Utf8) => {}
            Some(_) => {
                file_contents = file_contents
                    .iter()
                    .skip(bom.unwrap().len())
                    .copied()
                    .collect();
            }
        }

        log::trace!("Decoding file");
        let decoded = encoder
            .decode(&file_contents, DecoderTrap::Strict)
            .map_err(|e| anyhow!("Failed to decode file: {}", e))?;

        (bom, encoder, decoded)
    };

    // Sort the items so they're in order - ripgrep should give them to us in order anyway but we sort them here to
    // future-proof against any changes.
    // NOTE: we're sorting by the offset here with the assumption that no two Match items within one file will have
    // the same offset.
    items.sort_unstable_by_key(|i| i.offset());

    // Iterate over the items in _reverse_ order -> this is so offsets can stay the same even though we're making
    // changes to the string.
    let mut did_skip_replacement = false;
    for (i, item) in items.iter().rev().enumerate() {
        let offset = item.offset().unwrap();
        log::debug!("Item[{}] offset: {}", i, offset);

        // Iterate backwards so the offset doesn't change as we make replacements.
        for (i, sub_item) in item
            .sub_items()
            .iter()
            .rev()
            .filter(|s| s.should_replace)
            .enumerate()
        {
            let SubMatch { range, text } = &sub_item.sub_match;
            log::debug!("SubMatch[{}] range: {:?}, data: \"{}\"", i, range, text);

            let normalised_range = (offset + range.start)..(offset + range.end);
            let str_to_remove = &file_as_str[normalised_range.clone()];
            let matched_bytes = text.to_vec();

            if str_to_remove.as_bytes() == matched_bytes.as_slice() {
                let removed_str = str_to_remove.to_string();
                file_as_str.replace_range(normalised_range, &criteria.text);

                log::debug!(
                    "Replacement - reported line: {:?}, removed: \"{}\", added: \"{}\"",
                    item.line_number(),
                    removed_str,
                    criteria.text
                );
            } else {
                log::warn!("Matched bytes do not match bytes to replace!");
                log::warn!("\tFile: \"{}\"", path_buf.display());
                log::warn!("\tMatch: data=\"{}\", bytes={:?}", text, matched_bytes);
                log::warn!("\tOffset: {}", offset + range.start);
                did_skip_replacement = true;
            }
        }
    }

    // Convert back into the detected encoding.
    log::trace!("Re-encoding file");
    let replaced_contents = encoder
        .encode(&file_as_str, EncoderTrap::Strict)
        .map_err(|e| anyhow!("Failed to encode replaced string: {}", e))?;

    // Create a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    let temp_file_path = temp_file.path().display().to_string();
    log::debug!("Creating temporary file: {}", temp_file_path);

    // Write a BOM if one existed beforehand.
    if let Some(bom) = bom {
        // NOTE: we don't strip a UTF8 BOM, because ripgrep doesn't either therefore no need to re-write one
        // See: https://github.com/BurntSushi/ripgrep/issues/1638
        if !matches!(bom, Bom::Utf8) {
            let bom_bytes = bom.bytes();
            log::debug!("Writing BOM: {:?}", bom_bytes);
            temp_file.write_all(bom_bytes)?;
        }
    }

    // Write the replaced contents.
    log::debug!("Writing: {}", temp_file_path);
    temp_file.write_all(&replaced_contents)?;

    // Overwrite the original file with the patched temp file.
    log::debug!("Moving {} to {}", temp_file_path, path_buf.display());
    temp_file.into_temp_path().persist(&path_buf)?;

    Ok(did_skip_replacement)
}

pub fn perform_replacements(criteria: ReplacementCriteria) -> Result<()> {
    log::trace!("--- PERFORM REPLACEMENTS ---");
    log::debug!("Replacement text: \"{}\"", criteria.text);

    let rg_encoding = RgEncoding::from(&criteria.encoding);
    log::debug!("User passed encoding: {:?}", rg_encoding);

    // Group items by their file so we only open each file once.
    let mut did_skip_replacement = false;
    for meta in criteria.as_map() {
        match perform_replacements_in_file(&criteria, &rg_encoding, meta) {
            Ok(did_skip) => {
                if did_skip {
                    did_skip_replacement = true
                }
            }
            Err(e) => {
                did_skip_replacement = true;
                log::warn!("Failed to make all replacements: {}", e);
                eprintln!("Failed to make all replacements: {}", e);
                continue;
            }
        }
    }

    if did_skip_replacement {
        log::warn!("Failed to perform all replacements");
        Err(anyhow!("Failed to perform all replacements, see log"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};
    use std::path::PathBuf;

    use base64_simd::STANDARD as base64;
    use pretty_assertions::assert_eq;
    use tempfile::NamedTempFile;

    use crate::model::*;
    use crate::replace::perform_replacements;
    use crate::rg::de::test_utilities::RgMessageBuilder;
    use crate::rg::de::{Duration, RgMessageKind, Stats, SubMatch};
    use crate::ui::line::*;

    macro_rules! temp_item {
        ($offset:expr, $lines:expr, $submatches:expr) => {{
            let p = temp_file!($lines);
            let item = Item::new(
                0,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(p.to_string_lossy().to_string())
                    .with_lines_text($lines.to_string())
                    .with_submatches($submatches)
                    .with_offset($offset)
                    .build(),
            );

            (item, p)
        }};
    }

    // NOTE: due to permission issues on Windows platforms, we need to first "keep" the temporary files otherwise
    // we cannot atomically replace them. See https://github.com/Stebalien/tempfile/issues/131
    macro_rules! temp_file {
        (bytes, $content:expr) => {{
            let mut file = NamedTempFile::new().unwrap();
            file.write_all($content).unwrap();
            // NOTE: we *must* drop the file here, otherwise Windows will fail with permissions errors
            let (_, p) = file.keep().unwrap();
            p
        }};
        ($content:expr) => {
            temp_file!(bytes, $content.as_bytes())
        };
    }

    #[test]
    fn it_performs_replacements_only_on_match_items() {
        let text = "foo bar baz";
        let build_item = |kind, p: &PathBuf| {
            Item::new(
                0,
                RgMessageBuilder::new(kind)
                    .with_path_text(p.to_string_lossy())
                    .with_lines_text(text)
                    .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
                    .with_stats(Stats::new())
                    .with_elapsed_total(Duration::new())
                    .with_offset(0)
                    .build(),
            )
        };

        let p1 = temp_file!(text);
        let p2 = temp_file!(text);
        let p3 = temp_file!(text);
        let p4 = temp_file!(text);
        let p5 = temp_file!(text);

        let items = vec![
            build_item(RgMessageKind::Begin, &p1),
            build_item(RgMessageKind::Context, &p2),
            build_item(RgMessageKind::Match, &p3),
            build_item(RgMessageKind::End, &p4),
            build_item(RgMessageKind::Summary, &p5),
        ];

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(fs::read_to_string(p1).unwrap(), text);
        assert_eq!(fs::read_to_string(p2).unwrap(), text);
        assert_eq!(fs::read_to_string(p3).unwrap(), "NEW_VALUE bar baz");
        assert_eq!(fs::read_to_string(p4).unwrap(), text);
        assert_eq!(fs::read_to_string(p5).unwrap(), text);
    }

    #[test]
    fn it_performs_replacements_in_separate_files() {
        let (item1, p1) = temp_item!(0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]);
        let (item2, p2) = temp_item!(0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]);
        let (item3, p3) = temp_item!(0, "bar baz foo", vec![SubMatch::new_text("foo", 8..11)]);

        let items = vec![item1, item2, item3];
        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(fs::read_to_string(p1).unwrap(), "NEW_VALUE bar baz");
        assert_eq!(fs::read_to_string(p2).unwrap(), "baz NEW_VALUE bar");
        assert_eq!(fs::read_to_string(p3).unwrap(), "bar baz NEW_VALUE");
    }

    #[test]
    fn it_does_not_replace_deselected_matches() {
        let (item1, p1) = temp_item!(0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]);
        let (item2, p2) = temp_item!(0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]);
        let (item3, p3) = temp_item!(0, "bar baz foo", vec![SubMatch::new_text("foo", 8..11)]);

        let mut items = vec![item1, item2, item3];

        items[0].set_should_replace(0, false);
        items[1].set_should_replace(0, true);
        items[2].set_should_replace(0, false);

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(fs::read_to_string(p1).unwrap(), "foo bar baz");
        assert_eq!(fs::read_to_string(p2).unwrap(), "baz NEW_VALUE bar");
        assert_eq!(fs::read_to_string(p3).unwrap(), "bar baz foo");
    }

    #[test]
    fn it_performs_multiple_replacements_one_file() {
        let (item, p) = temp_item!(
            0,
            "foo bar baz",
            vec![
                SubMatch::new_text("foo", 0..3),
                SubMatch::new_text("bar", 4..7),
                SubMatch::new_text("baz", 8..11),
            ]
        );

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", vec![item])).unwrap();
        assert_eq!(
            fs::read_to_string(p).unwrap(),
            "NEW_VALUE NEW_VALUE NEW_VALUE"
        );
    }

    #[test]
    fn it_performs_replacements_on_multiple_lines() {
        let p = temp_file!("foo bar baz\n...\nbaz foo bar\n...\nbar baz foo");

        let path_string = p.to_string_lossy();
        let items = vec![
            Item::new(
                0,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
                    .with_lines_text("foo bar baz\n")
                    .with_offset(0)
                    .build(),
            ),
            Item::new(
                1,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("foo", 4..7)])
                    .with_lines_text("baz foo bar\n")
                    .with_offset(16)
                    .build(),
            ),
            Item::new(
                2,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("foo", 8..11)])
                    .with_lines_text("bar baz foo")
                    .with_offset(32)
                    .build(),
            ),
        ];

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(
            fs::read_to_string(p).unwrap(),
            "NEW_VALUE bar baz\n...\nbaz NEW_VALUE bar\n...\nbar baz NEW_VALUE"
        );
    }

    #[test]
    fn it_performs_replacements_on_multiline_matches() {
        let p = temp_file!("foo bar baz\n...\nbaz 1\n22\n333 bar\n...\nbar 4444 foo");

        let path_string = p.to_string_lossy();
        let items = vec![
            Item::new(
                0,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("1\n22\n333", 4..12)])
                    .with_lines_text("baz 1\n22\n333 bar\n")
                    .with_offset(16)
                    .build(),
            ),
            Item::new(
                1,
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("4444", 4..8)])
                    .with_lines_text("bar 4444 foo")
                    .with_offset(37)
                    .build(),
            ),
        ];

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(
            fs::read_to_string(p).unwrap(),
            "foo bar baz\n...\nbaz NEW_VALUE bar\n...\nbar NEW_VALUE foo"
        );
    }

    // TODO: write a similar test for Windows/macOS systems
    #[test]
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    fn it_performs_replacements_files_with_non_utf8_paths_unix() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::path::PathBuf;
        use tempfile::tempdir;

        // Here, the values 0x66 and 0x6f correspond to 'f' and 'o'
        // respectively. The value 0x80 is a lone continuation byte, invalid
        // in a UTF-8 sequence.
        let invalid_file_name_bytes = [0x66, 0x6f, 0x80, 0x6f];
        let invalid_file_name = OsStr::from_bytes(&invalid_file_name_bytes[..]);

        let d = tempdir().unwrap();
        let p = PathBuf::from(d.path()).join(invalid_file_name);
        let lines = "hello earth";
        fs::write(&p, lines.as_bytes()).unwrap();

        let item = Item::new(
            0,
            RgMessageBuilder::new(RgMessageKind::Match)
                .with_path_base64(base64.encode_to_string(p.as_os_str().as_bytes()))
                .with_lines_text(lines.to_string())
                .with_submatches(vec![SubMatch::new_text("o", 4..5)])
                .with_offset(0)
                .build(),
        );

        perform_replacements(ReplacementCriteria::new(" on", vec![item])).unwrap();
        assert_eq!(fs::read_to_string(p).unwrap(), "hell on earth");
    }

    // Encodings

    macro_rules! simple_test {
        ($name:ident, $src:expr, $dst:expr, ($needle:expr, $replace:expr), $submatches:expr) => {
            #[test]
            fn $name() {
                let src_bytes = hex::decode($src).unwrap();

                let p = temp_file!(bytes, &src_bytes);

                let items: Vec<Item> = $submatches
                    .iter()
                    .map(|(offset, range)| {
                        Item::new(
                            0,
                            RgMessageBuilder::new(RgMessageKind::Match)
                                .with_path_text(p.to_string_lossy())
                                .with_lines_text(&format!("{}\n", $needle))
                                .with_submatches(vec![SubMatch::new_text($needle, range.clone())])
                                .with_offset(*offset)
                                .build(),
                        )
                    })
                    .collect();

                perform_replacements(ReplacementCriteria::new($replace, items)).unwrap();

                // Read file bytes.
                let mut file_bytes = vec![];
                OpenOptions::new()
                    .read(true)
                    .open(p)
                    .unwrap()
                    .read_to_end(&mut file_bytes)
                    .unwrap();

                // Check real bytes are the same as expected bytes.
                assert_eq!(file_bytes, hex::decode($dst).unwrap());
            }
        };
    }

    // The following are generated with:
    //   printf "<BOM>%s" $(printf "foo bar baz\n...\nbaz foo bar\n...\nbar baz foo" | iconv -f UTF8 -t <ENCODING> | xxd -p -c 128)
    // printf "efbbbf%s" $(printf "RUST bar baz\n...\nbaz RUST bar\n...\nbar baz RUST" | iconv -f UTF8 -t UTF8 | xxd -p -c 128)

    const UTF8_FOO: &str =
        "666f6f206261722062617a0a2e2e2e0a62617a20666f6f206261720a2e2e2e0a6261722062617a20666f6f";
    const UTF8BOM_FOO: &str = "efbbbf666f6f206261722062617a0a2e2e2e0a62617a20666f6f206261720a2e2e2e0a6261722062617a20666f6f";
    const UTF16BE_FOO: &str = "feff0066006f006f0020006200610072002000620061007a000a002e002e002e000a00620061007a00200066006f006f0020006200610072000a002e002e002e000a006200610072002000620061007a00200066006f006f";
    const UTF16LE_FOO: &str = "fffe66006f006f0020006200610072002000620061007a000a002e002e002e000a00620061007a00200066006f006f0020006200610072000a002e002e002e000a006200610072002000620061007a00200066006f006f00";

    // The following are generated with:
    //   printf "<BOM>%s" $(printf "RUST bar baz\n...\nbaz RUST bar\n...\nbar baz RUST" | iconv -f UTF8 -t <ENCODING> | xxd -p -c 128)

    const UTF8_RUST: &str = "52555354206261722062617a0a2e2e2e0a62617a2052555354206261720a2e2e2e0a6261722062617a2052555354";
    const UTF8BOM_RUST: &str = "efbbbf52555354206261722062617a0a2e2e2e0a62617a2052555354206261720a2e2e2e0a6261722062617a2052555354";
    const UTF16BE_RUST: &str = "feff00520055005300540020006200610072002000620061007a000a002e002e002e000a00620061007a002000520055005300540020006200610072000a002e002e002e000a006200610072002000620061007a00200052005500530054";
    const UTF16LE_RUST: &str = "fffe520055005300540020006200610072002000620061007a000a002e002e002e000a00620061007a002000520055005300540020006200610072000a002e002e002e000a006200610072002000620061007a0020005200550053005400";

    // The following are generated with:
    //   printf "<BOM>%s" $(printf "A bar baz\n...\nbaz A bar\n...\nbar baz A" | iconv -f UTF8 -t <ENCODING> | xxd -p -c 128)

    const UTF8_A: &str =
        "41206261722062617a0a2e2e2e0a62617a2041206261720a2e2e2e0a6261722062617a2041";
    const UTF8BOM_A: &str =
        "efbbbf41206261722062617a0a2e2e2e0a62617a2041206261720a2e2e2e0a6261722062617a2041";
    const UTF16BE_A: &str = "feff00410020006200610072002000620061007a000a002e002e002e000a00620061007a002000410020006200610072000a002e002e002e000a006200610072002000620061007a00200041";
    const UTF16LE_A: &str = "fffe410020006200610072002000620061007a000a002e002e002e000a00620061007a002000410020006200610072000a002e002e002e000a006200610072002000620061007a0020004100";

    simple_test!(
        multiline_longer_utf8,
        UTF8_FOO,
        UTF8_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_longer_utf8_bom,
        UTF8BOM_FOO,
        UTF8BOM_RUST,
        ("foo", "RUST"),
        &[(0, 3..6), (19, 4..7), (35, 8..11)]
    );

    simple_test!(
        multiline_longer_utf16be,
        UTF16BE_FOO,
        UTF16BE_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_longer_utf16le,
        UTF16LE_FOO,
        UTF16LE_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_shorter_utf8,
        UTF8_FOO,
        UTF8_A,
        ("foo", "A"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_shorter_utf8_bom,
        UTF8BOM_FOO,
        UTF8BOM_A,
        ("foo", "A"),
        &[(0, 3..6), (19, 4..7), (35, 8..11)]
    );

    simple_test!(
        multiline_shorter_utf16be,
        UTF16BE_FOO,
        UTF16BE_A,
        ("foo", "A"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_shorter_utf16le,
        UTF16LE_FOO,
        UTF16LE_A,
        ("foo", "A"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );
}
