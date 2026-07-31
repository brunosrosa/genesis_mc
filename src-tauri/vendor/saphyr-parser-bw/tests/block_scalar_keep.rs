// 6FWR: Block Scalar Keep (|+)
use saphyr_parser_bw as saphyr_parser;
use saphyr_parser::{Event, Parser};

#[test]
fn case_6fwr_keep_space() {
    // Suite expectation: "ab\n\n \n" \u{2014} the final kept line contains a single space.
    let yaml = "--- |+\n ab\n\n \n...\n";
    let mut parser = Parser::new_from_str(yaml);

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
                // "ab\n\n\n"|Literal|0|Span { start: Marker { index: 8, line: 2, col: 1 }, end: Marker { index: 14, line: 5, col: 0 } }
                println!("{:?}|{:?}|{:?}|{:?}", cow, style, size, span);
            },
            Ok((_event, _)) => {}
        }
    }
}