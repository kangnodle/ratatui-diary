use org_render::render_to_text;
use ratatui::style::Modifier;

fn rendered(org: &str) -> String { render_to_text(org).lines.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n") }

#[test]
fn headlines_use_repeating_org_bullets_and_keep_their_titles() {
    let org = "* 오늘\n** TODO [#A] 작업 :diary:\n*** 배운 것\n**** 내일\n***** 다섯 단계";
    assert_eq!(rendered(org), "◉ 오늘\n ○ TODO [#A] 작업 :diary:\n  ✸ 배운 것\n   ✿ 내일\n    ◉ 다섯 단계");
    assert!(render_to_text(org).lines.iter().all(|line| line.style.add_modifier.contains(Modifier::BOLD)));
}

#[test]
fn body_lists_markup_and_non_headlines_are_preserved() {
    let org = "#+title: 일기\n\n본문 *강조* /기울임/\n- [X] 할 일\n  * 들여쓴 목록\n*bold*\n**별표\n*\n*\t탭\n[[https://orgmode.org][Org]]\n";
    assert_eq!(rendered(org), org);
}

#[test]
fn empty_documents_blank_headlines_and_crlf_are_supported() {
    assert_eq!(rendered(""), "");
    assert_eq!(render_to_text("").lines.len(), 1);
    assert_eq!(rendered("* \r\n\r\n** 한글 café 𠮷\r\n"), "◉ \n\n ○ 한글 café 𠮷\n");
}

#[test]
fn literal_blocks_do_not_turn_stars_into_headlines() {
    for block in ["src", "example", "export", "comment"] {
        let literal = format!(
            "  #+BEGIN_{} rust\n* literal\n** literal\n#+end_other\n* still literal\n  #+END_{}",
            block.to_uppercase(),
            block.to_uppercase()
        );
        assert_eq!(rendered(&format!("{literal}\n* 제목")), format!("{literal}\n◉ 제목"));
    }
}

#[test]
fn unfinished_literal_blocks_stay_verbatim() {
    let org = "#+begin_src\n* literal\n** literal";
    assert_eq!(rendered(org), org);
}
