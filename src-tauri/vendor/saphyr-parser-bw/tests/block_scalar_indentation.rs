use saphyr_parser_bw as saphyr_parser;
use saphyr_parser::{Event, Parser};

#[test]
fn yaml_5llu_block_scalar_wrong_indent_should_fail() {
    // It is ONLY an error for any of the leading empty lines to contain MORE spaces than the first non-empty line.
    let yaml_ok = "block scalar: >\n\n  \n   \n    invalid\n";

    let mut parser = Parser::new_from_str(yaml_ok);

    while let Some(next) = parser.next() {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                break; // fine
            }
            Err(err) => {
                assert!(false, "{} reported for valid YAML", err);
                break;
            }
            _ => {}
        }
    }

    // It IS an error for any of the leading empty lines to contain MORE spaces than the first non-empty line.
    let yaml_invalid = "block scalar: >\n\n                             \n   \n    invalid\n";

    let mut parser = Parser::new_from_str(yaml_invalid);

    while let Some(next) = parser.next() {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                assert!(false, "Document end before any error, invalid YAML");
            }
            Err(_err) => {
                break; // fine
            }
            _ => {}
        }
    }
}
