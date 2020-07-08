use std::fs::{self, OpenOptions};
use std::io::{Read, Write};

use anyhow::Result;
use encoding::label::encoding_from_whatwg_label;
use encoding::{DecoderTrap, EncoderTrap};

use crate::encoding::{get_encoder, Bom};
use crate::model::{ReplacementAttempt, ReplacementCriteria, ReplacementResult};
use crate::rg::de::{RgMessageKind, SubMatch};
use crate::rg::RgEncoding;

// TODO: better error handling and messaging to the user when any of this fails
pub fn perform_replacements(criteria: ReplacementCriteria) -> Result<ReplacementResult> {
    // If we've been passed an encoding, then try to create an encoder from it.
    let rg_encoding = match criteria.encoding.as_ref() {
        Some(label) => {
            if label == "none" {
                RgEncoding::NoneExplicit
            } else {
                encoding_from_whatwg_label(label)
                    .map_or_else(|| RgEncoding::None, |e| RgEncoding::Some(e))
            }
        }
        None => RgEncoding::None,
    };

    Ok(criteria
        .items
        .iter()
        // Iterate backwards so the offset doesn't change as we make replacements.
        .rev()
        // The only item kind we replace is the Match kind.
        .filter(|item| matches!(item.kind, RgMessageKind::Match) && item.should_replace)
        // Perform the replacement on each match.
        .fold(ReplacementResult::new(&criteria.text), |mut res, item| {
            let file_path = item.path().expect("match item did not have a path!");

            // TODO: don't read file completely into memory, but use a buffered approach instead
            let mut file_contents = vec![];
            OpenOptions::new()
                .read(true)
                .open(&file_path)
                .unwrap()
                .read_to_end(&mut file_contents)
                .unwrap();

            // Search for a BOM and attempt to detect file encoding.
            let (bom, encoder) = get_encoder(&file_contents, &rg_encoding);

            // Strip the BOM before we decode.
            // NOTE: we don't strip a UTF8 BOM, because ripgrep doesn't either
            // See: https://github.com/BurntSushi/ripgrep/issues/1638
            if bom.is_some() && !matches!(bom, Some(Bom::Utf8)) {
                file_contents = file_contents
                    .iter()
                    .skip(bom.unwrap().len())
                    .copied()
                    .collect();
            }

            // Decode file.
            let mut file_as_str = match encoder.decode(&file_contents, DecoderTrap::Strict) {
                Ok(s) => s,
                Err(e) => {
                    res.add_replacement(
                        &file_path,
                        vec![ReplacementAttempt::Failure(format!(
                            "Failed to decode file: {}",
                            e
                        ))],
                        encoder.name(),
                    );
                    return res;
                }
            };

            // Replace matches within the file contents with the given `replacement` string.
            let replaced_matches = item.matches().map_or_else(
                || vec![],
                |submatches| {
                    let offset = item.offset().unwrap();

                    // Iterate backwards so the offset doesn't change as we make replacements.
                    submatches
                        .iter()
                        .rev()
                        .map(|SubMatch { text, range }| {
                            let normalised_range = (offset + range.start)..(offset + range.end);
                            dbg!(&normalised_range);
                            let str_to_remove = &file_as_str[normalised_range.clone()];
                            if str_to_remove.as_bytes() == text.to_vec().as_slice() {
                                let removed_str = str_to_remove.to_string();
                                file_as_str.replace_range(normalised_range, &criteria.text);
                                ReplacementAttempt::Success(removed_str)
                            } else {
                                ReplacementAttempt::Failure(format!(
                                    "Matched bytes do not match bytes to replace in {}@{}!",
                                    file_path.display(),
                                    offset + range.start,
                                ))
                            }
                        })
                        .collect()
                },
            );

            // Convert back into the detected encoding.
            let replaced_contents = match encoder.encode(&file_as_str, EncoderTrap::Strict) {
                Ok(bytes) => bytes,
                Err(e) => {
                    res.add_replacement(
                        &file_path,
                        vec![ReplacementAttempt::Failure(format!(
                            "Failed to encode replaced string: {}",
                            e
                        ))],
                        encoder.name(),
                    );
                    return res;
                }
            };

            // Create a temporary file.
            #[cfg(not(windows))]
            let temp_file_path = &file_path.with_extension("rgr");
            // FIXME: for reasons unknown to me Windows fails with permissions errors if we try to create a new file
            // next to the original, so for now, we don't create a temporary file.
            #[cfg(windows)]
            let temp_file_path = &file_path;

            // Write modified string into a temporary file.
            let mut dest_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&temp_file_path)
                .unwrap();

            // Write a BOM if one existed beforehand.
            if let Some(bom) = bom {
                // NOTE: we don't strip a UTF8 BOM, because ripgrep doesn't either therefore no need to re-write one
                // See: https://github.com/BurntSushi/ripgrep/issues/1638
                if !matches!(bom, Bom::Utf8) {
                    dest_file.write_all(bom.bytes()).unwrap();
                }
            }
            dest_file.write_all(&replaced_contents).unwrap();

            // Overwrite the original file with the patched temp file.
            #[cfg(not(windows))]
            fs::rename(temp_file_path, &file_path).unwrap();

            // Add the results of the replacement.
            // TODO: potentially log these as they occur to avoid hanging on a lot of files.
            res.add_replacement(&file_path, replaced_matches, encoder.name());
            res
        }))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, OpenOptions};
    use std::io::{Read, Write};

    use pretty_assertions::assert_eq;
    use tempfile::NamedTempFile;

    use crate::model::*;
    use crate::replace::perform_replacements;
    use crate::rg::de::test_utilities::RgMessageBuilder;
    use crate::rg::de::{Duration, RgMessageKind, Stats, SubMatch};

    fn temp_rg_msg(
        mut f: &NamedTempFile,
        offset: usize,
        lines: impl AsRef<str>,
        submatches: Vec<SubMatch>,
    ) -> Item {
        f.write_all(lines.as_ref().as_bytes()).unwrap();

        Item::new(
            RgMessageBuilder::new(RgMessageKind::Match)
                .with_path_text(f.path().to_string_lossy().to_string())
                .with_lines_text(lines.as_ref().to_string())
                .with_submatches(submatches)
                .with_offset(offset)
                .build(),
        )
    }

    #[test]
    fn it_performs_replacements_only_on_match_items() {
        let text = "foo bar baz";
        let build_item = |kind, mut f: &NamedTempFile| {
            f.write_all(text.as_bytes()).unwrap();
            Item::new(
                RgMessageBuilder::new(kind)
                    .with_path_text(f.path().to_string_lossy())
                    .with_lines_text(text)
                    .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
                    .with_stats(Stats::new())
                    .with_elapsed_total(Duration::new())
                    .with_offset(0)
                    .build(),
            )
        };

        let f1 = NamedTempFile::new().unwrap();
        let f2 = NamedTempFile::new().unwrap();
        let f3 = NamedTempFile::new().unwrap();
        let f4 = NamedTempFile::new().unwrap();
        let f5 = NamedTempFile::new().unwrap();

        let items = vec![
            build_item(RgMessageKind::Begin, &f1),
            build_item(RgMessageKind::Context, &f2),
            build_item(RgMessageKind::Match, &f3),
            build_item(RgMessageKind::End, &f4),
            build_item(RgMessageKind::Summary, &f5),
        ];

        let result = perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(result.replacements.len(), 1);

        assert_eq!(fs::read_to_string(f1.path()).unwrap(), text);
        assert_eq!(fs::read_to_string(f2.path()).unwrap(), text);
        assert_eq!(fs::read_to_string(f3.path()).unwrap(), "NEW_VALUE bar baz");
        assert_eq!(fs::read_to_string(f4.path()).unwrap(), text);
        assert_eq!(fs::read_to_string(f5.path()).unwrap(), text);
    }

    #[test]
    fn it_performs_replacements_in_separate_files() {
        let f1 = NamedTempFile::new().unwrap();
        let f2 = NamedTempFile::new().unwrap();
        let f3 = NamedTempFile::new().unwrap();

        let items = vec![
            temp_rg_msg(&f1, 0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
            temp_rg_msg(&f2, 0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
            temp_rg_msg(
                &f3,
                0,
                "bar baz foo",
                vec![SubMatch::new_text("foo", 8..11)],
            ),
        ];

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(fs::read_to_string(f1.path()).unwrap(), "NEW_VALUE bar baz");
        assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz NEW_VALUE bar");
        assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz NEW_VALUE");
    }

    #[test]
    fn it_does_not_replace_deselected_matches() {
        let f1 = NamedTempFile::new().unwrap();
        let f2 = NamedTempFile::new().unwrap();
        let f3 = NamedTempFile::new().unwrap();

        let mut items = vec![
            temp_rg_msg(&f1, 0, "foo bar baz", vec![SubMatch::new_text("foo", 0..3)]),
            temp_rg_msg(&f2, 0, "baz foo bar", vec![SubMatch::new_text("foo", 4..7)]),
            temp_rg_msg(
                &f3,
                0,
                "bar baz foo",
                vec![SubMatch::new_text("foo", 8..11)],
            ),
        ];

        items[0].should_replace = false;
        items[1].should_replace = true;
        items[2].should_replace = false;

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", items)).unwrap();
        assert_eq!(fs::read_to_string(f1.path()).unwrap(), "foo bar baz");
        assert_eq!(fs::read_to_string(f2.path()).unwrap(), "baz NEW_VALUE bar");
        assert_eq!(fs::read_to_string(f3.path()).unwrap(), "bar baz foo");
    }

    #[test]
    fn it_performs_multiple_replacements_one_file() {
        let f = NamedTempFile::new().unwrap();
        let item = temp_rg_msg(
            &f,
            0,
            "foo bar baz",
            vec![
                SubMatch::new_text("foo", 0..3),
                SubMatch::new_text("bar", 4..7),
                SubMatch::new_text("baz", 8..11),
            ],
        );

        perform_replacements(ReplacementCriteria::new("NEW_VALUE", vec![item])).unwrap();
        assert_eq!(
            fs::read_to_string(f.path()).unwrap(),
            "NEW_VALUE NEW_VALUE NEW_VALUE"
        );
    }

    #[test]
    fn it_performs_replacements_on_multiple_lines() {
        let mut f = NamedTempFile::new().unwrap();

        f.write_all(b"foo bar baz\n...\nbaz foo bar\n...\nbar baz foo")
            .unwrap();

        let path_string = f.path().to_string_lossy();
        let items = vec![
            Item::new(
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("foo", 0..3)])
                    .with_lines_text("foo bar baz\n")
                    .with_offset(0)
                    .build(),
            ),
            Item::new(
                RgMessageBuilder::new(RgMessageKind::Match)
                    .with_path_text(&path_string)
                    .with_submatches(vec![SubMatch::new_text("foo", 4..7)])
                    .with_lines_text("baz foo bar\n")
                    .with_offset(16)
                    .build(),
            ),
            Item::new(
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
            fs::read_to_string(f.path()).unwrap(),
            "NEW_VALUE bar baz\n...\nbaz NEW_VALUE bar\n...\nbar baz NEW_VALUE"
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
            RgMessageBuilder::new(RgMessageKind::Match)
                .with_path_base64(base64::encode(p.as_os_str().as_bytes()))
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

                let mut f = NamedTempFile::new().unwrap();
                f.write_all(&src_bytes).unwrap();

                let items: Vec<Item> = $submatches
                    .iter()
                    .map(|(offset, range)| {
                        Item::new(
                            RgMessageBuilder::new(RgMessageKind::Match)
                                .with_path_text(f.path().to_string_lossy())
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
                    .open(f.path())
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
    const UTF8_FOO_BOM: &str = "efbbbf666f6f206261722062617a0a2e2e2e0a62617a20666f6f206261720a2e2e2e0a6261722062617a20666f6f";
    const UTF16BE_FOO: &str = "feff0066006f006f0020006200610072002000620061007a000a002e002e002e000a00620061007a00200066006f006f0020006200610072000a002e002e002e000a006200610072002000620061007a00200066006f006f";
    const UTF16LE_FOO: &str = "fffe66006f006f0020006200610072002000620061007a000a002e002e002e000a00620061007a00200066006f006f0020006200610072000a002e002e002e000a006200610072002000620061007a00200066006f006f00";

    // The following are generated with:
    //   printf "<BOM>%s" $(printf "RUST bar baz\n...\nbaz RUST bar\n...\nbar baz RUST" | iconv -f UTF8 -t <ENCODING> | xxd -p -c 128)

    const UTF8_RUST: &str = "52555354206261722062617a0a2e2e2e0a62617a2052555354206261720a2e2e2e0a6261722062617a2052555354";
    const UTF8_RUST_BOM: &str = "efbbbf52555354206261722062617a0a2e2e2e0a62617a2052555354206261720a2e2e2e0a6261722062617a2052555354";
    const UTF16BE_RUST: &str = "feff00520055005300540020006200610072002000620061007a000a002e002e002e000a00620061007a002000520055005300540020006200610072000a002e002e002e000a006200610072002000620061007a00200052005500530054";
    const UTF16LE_RUST: &str = "fffe520055005300540020006200610072002000620061007a000a002e002e002e000a00620061007a002000520055005300540020006200610072000a002e002e002e000a006200610072002000620061007a0020005200550053005400";

    simple_test!(
        multiline_utf8,
        UTF8_FOO,
        UTF8_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_utf8_bom,
        UTF8_FOO_BOM,
        UTF8_RUST_BOM,
        ("foo", "RUST"),
        &[(0, 3..6), (19, 4..7), (35, 8..11)]
    );

    simple_test!(
        multiline_utf16be,
        UTF16BE_FOO,
        UTF16BE_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );

    simple_test!(
        multiline_utf16le,
        UTF16LE_FOO,
        UTF16LE_RUST,
        ("foo", "RUST"),
        &[(0, 0..3), (16, 4..7), (32, 8..11)]
    );
}
