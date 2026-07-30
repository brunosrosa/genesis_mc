use saphyr_parser_bw as saphyr_parser;
use saphyr_parser::{Event, Parser};

#[test]
fn yaml_xv9v_empty_lines_and_chomping() {
    let yaml = r#"
    Folding: "Empty line
             as a line feed"
    Chomping: |
        Clipped empty lines

"#;

    let mut parser = Parser::new_from_str(&yaml);

    while let Some(next) = parser.next() {
        match next {
            Ok((Event::DocumentEnd, _)) => {
                break; // fine
            }
            Err(err) => {
                assert!(false, "{} reported for valid YAML", err);
                break;
            }
            Ok((Event::Scalar(cow, style, size, _tag), span)) => {
                println!("{:?}|{:?}|{:?}|{:?}", cow, style, size, span);
            },
            _ => {}
        }
    }
}
