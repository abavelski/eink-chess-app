// Task 08: exercise untouched Kindle and Kobo bytes through the simulated Cobalt app.
// Included from main.rs's test module so the events use the production action handlers.
const KINDLE_BOOK: &[u8] =
    include_bytes!("../tests/fixtures/kindle-parity/pgn-converter/valid-book.json");
const KINDLE_V1: &[u8] = include_bytes!("../tests/fixtures/kindle-parity/puzzles.json");

#[test]
fn parity_rich_fixture_reader_actions_and_grading() {
    let (mut runner, _) = runner(Some(RICH_PUZZLES.to_vec()));
    assert_eq!(runner.app().puzzles[0].id, "rich-black-promotion");
    assert_eq!(
        runner.app().collections[0].title.as_deref(),
        Some("Task 20 rich-analysis contract")
    );
    let before = attempt_snapshot(runner.app());
    let document = project_analysis(&runner.app().puzzles[0]);
    assert!(document.text().contains("h1=Q+"));
    let tree = runner.app().puzzles[0].analysis.as_ref().unwrap();
    let alternative = tree.by_id("n2").unwrap();
    let alternative_fen = tree.node(alternative).unwrap().fen().to_owned();
    let (run, _) = document
        .runs
        .iter()
        .enumerate()
        .find(|(_, item)| item.target == Some(alternative))
        .expect("the alternative has a visible link");
    let pages = paginate(&document);
    let page = pages
        .iter()
        .position(|page| page.fragments.iter().any(|item| item.source_run == run))
        .unwrap();

    assert_no_progress_write(&runner.action(action_id(ANALYSIS_TOGGLE)));
    while runner.app().analysis_browser.page() < page {
        assert_no_progress_write(&runner.page_turn(true));
    }
    let epoch = runner.app().analysis_browser.epoch();
    assert_no_progress_write(&runner.action(action_id(&link_action(epoch, run))));
    assert_eq!(
        runner.app().analysis_browser.preview_fen(),
        Some(alternative_fen.as_str())
    );
    assert_eq!(
        runner.app().displayed_board(),
        &Board::from_fen(&alternative_fen).unwrap()
    );
    assert_eq!(attempt_snapshot(runner.app()), before);
    assert_no_progress_write(&runner.action(action_id(ANALYSIS_CLOSE)));
    assert_eq!(attempt_snapshot(runner.app()), before);
    assert_eq!(runner.app().displayed_board(), &before.board);

    assert_no_progress_write(&play(&mut runner, "h2h1n"));
    assert_eq!(runner.app().solution_feedback, Some(SolutionFeedback::Wrong));
    assert!(!runner.app().current_is_solved());
    let saved = play(&mut runner, "h2h1q");
    assert_eq!(
        runner.app().solution_feedback,
        Some(SolutionFeedback::Complete)
    );
    assert!(runner.app().current_is_solved());
    assert!(progress_bytes(&saved).is_some());
}

#[test]
fn parity_book_picker_nested_refs_exact_fens_and_main_lines() {
    use crate::pgn_projection::RunOrigin;

    let (mut runner, _) = runner_with_collections(vec![
        (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
        ("puzzles-book.json", KINDLE_BOOK.to_vec()),
    ]);
    let commands = switch_collection(&mut runner, "puzzles-book.json", KINDLE_BOOK);
    assert!(progress_bytes(&commands).is_some());
    ack_progress_save(&mut runner);
    assert_eq!(
        runner.app().active_collection.as_deref(),
        Some("puzzles-book.json")
    );
    assert_eq!(runner.app().puzzles.len(), 3);

    for index in 0..3 {
        if index > 0 {
            runner.page_turn(true);
            ack_progress_save(&mut runner);
        }
        assert_eq!(runner.app().puzzle_index, index);
        let before = attempt_snapshot(runner.app());
        let puzzle = &runner.app().puzzles[index];
        let document = project_analysis(puzzle);
        let pages = paginate(&document);
        assert!(!pages.is_empty());
        if index == 0 {
            assert!(document
                .runs
                .iter()
                .any(|run| run.origin == RunOrigin::ExplicitMoveRef));
            assert!(document.text().contains("Nd5/e2e4"));
        }
        let refs: Vec<_> = document
            .runs
            .iter()
            .enumerate()
            .filter_map(|(run, item)| item.target.map(|node| (run, node)))
            .collect();
        assert!(refs.len() >= 3);
        assert_no_progress_write(&runner.action(action_id(ANALYSIS_TOGGLE)));
        let epoch = runner.app().analysis_browser.epoch();
        for &(run, node) in &refs {
            let page = pages
                .iter()
                .position(|page| {
                    page.fragments
                        .iter()
                        .any(|fragment| fragment.source_run == run && fragment.target == Some(node))
                })
                .unwrap();
            while runner.app().analysis_browser.page() < page {
                assert_no_progress_write(&runner.action(action_id(ANALYSIS_NEXT)));
            }
            while runner.app().analysis_browser.page() > page {
                assert_no_progress_write(&runner.action(action_id(ANALYSIS_PREVIOUS)));
            }
            assert_no_progress_write(&runner.action(action_id(&link_action(epoch, run))));
            let fen = runner.app().puzzles[index]
                .analysis
                .as_ref()
                .unwrap()
                .node(node)
                .unwrap()
                .fen();
            assert_eq!(runner.app().analysis_browser.preview_fen(), Some(fen));
            assert_eq!(runner.app().displayed_board(), &Board::from_fen(fen).unwrap());
            assert_eq!(attempt_snapshot(runner.app()), before);
        }
        assert_no_progress_write(&runner.action(action_id(ANALYSIS_CLOSE)));
        assert_eq!(attempt_snapshot(runner.app()), before);
    }

    runner.page_turn(false);
    runner.page_turn(false);
    assert_eq!(runner.app().puzzle_index, 0);
    // Two navigation events may enqueue a second save while the first is
    // in flight. Drain both acknowledgements before checking solve writes.
    while runner.app().progress_saving.is_some() {
        ack_progress_save(&mut runner);
    }
    let first = play(&mut runner, "e2e4");
    assert_no_progress_write(&first);
    assert_eq!(runner.app().solution_ply, 2);
    let completed = play(&mut runner, "g1f3");
    assert!(progress_bytes(&completed).is_some());
    assert!(runner.app().current_is_solved());
    assert!(runner.app().progress.is_solved("puzzles-book.json", "nested-001"));
    assert!(!runner.app().progress.is_solved(PUZZLES_FILE, "lichess-001cr"));
}

#[test]
fn parity_unchanged_legacy_files_preserve_board_modes_and_progress() {
    for raw in [EXAMPLE_PUZZLES, KINDLE_V1] {
        let (mut runner, _) = runner(Some(raw.to_vec()));
        assert_eq!(runner.app().puzzles[0].id, "lichess-001cr");
        runner.action(action_id(FLIP));
        assert!(runner.app().flipped);
        let commands = runner.action(action_id(MODE_TOGGLE));
        assert_no_progress_write(&commands);
        assert_eq!(runner.app().mode, BoardMode::FreeBoard);
        assert_no_progress_write(&play(&mut runner, "d7e8"));
        assert_eq!(runner.app().solution_ply, 0);
        assert!(!runner.app().current_is_solved());
        runner.action(action_id(RESET));
        runner.action(action_id(MODE_TOGGLE));
        assert_eq!(runner.app().mode, BoardMode::Solution);
        assert!(runner.app().flipped);
        let done = play(&mut runner, "d7e8");
        assert!(progress_bytes(&done).is_some());
        assert!(runner.app().current_is_solved());
        assert_eq!(runner.app().solution_feedback, Some(SolutionFeedback::Complete));
        assert_eq!(runner.app().puzzles[0].description.as_deref(),
            Some("Solution begins with d7e8.\nFollow the line one move at a time.\nReplay it to practice the idea."));
    }
}

#[test]
fn parity_invalid_picker_and_future_progress_never_destroy_active_data() {
    let invalid = include_bytes!("../tests/fixtures/kindle-parity/rich-analysis/invalid-cycle.json");
    for damaged_progress in [
        b"corrupted".to_vec(),
        br#"{"version":99,"files":{}}"#.to_vec(),
    ] {
        let (mut runner, _) = runner_with_state(
            vec![
                (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
                ("puzzles-invalid.json", invalid.to_vec()),
                ("puzzles-rich.json", RICH_PUZZLES.to_vec()),
            ],
            Some(damaged_progress),
        );
        assert!(!runner.app().progress_durable);
        let before = attempt_snapshot(runner.app());
        let commands = switch_collection(&mut runner, "puzzles-invalid.json", invalid);
        assert_no_progress_write(&commands);
        assert_eq!(runner.app().active_collection.as_deref(), Some(PUZZLES_FILE));
        assert_eq!(attempt_snapshot(runner.app()), before);
        assert!(runner.app().file_error.as_deref().unwrap().contains("puzzles-invalid.json"));
        let commands = switch_collection(&mut runner, "puzzles-rich.json", RICH_PUZZLES);
        assert_no_progress_write(&commands);
        assert_eq!(runner.app().puzzles[0].id, "rich-black-promotion");
        assert!(!runner.app().progress_durable);
    }
}

#[test]
fn parity_large_rich_book_uses_ranges_not_single_store_frame() {
    let mut value: serde_json::Value = serde_json::from_slice(RICH_PUZZLES).unwrap();
    value["puzzles"][0]["description"] =
        serde_json::Value::String("large-read-only-preface".repeat(20_000));
    let large = serde_json::to_vec(&value).unwrap();
    assert!(large.len() > 256 * 1024);
    assert!(large.len() <= large_collection::MAX_COLLECTION_BYTES);
    let key = "puzzles-large-rich.json";
    let (mut runner, _) = runner_with_collections(vec![
        (PUZZLES_FILE, EXAMPLE_PUZZLES.to_vec()),
        (key, RICH_PUZZLES.to_vec()),
    ]);
    let commands = select_with_range_reply(&mut runner, key, &large, None);
    let range_requests = commands
        .iter()
        .filter(|command| matches!(command, Command::Store(StoreRequest::ShelfRead { .. })))
        .count();
    assert!(range_requests >= 2 * large.len().div_ceil(large_collection::READ_CHUNK));
    assert!(commands.iter().all(|command| !matches!(
        command,
        Command::Store(StoreRequest::Save { key: name, .. }) if name == key
    )));
    assert_eq!(runner.app().active_collection.as_deref(), Some(key));
    assert_eq!(runner.app().puzzles[0].id, "rich-black-promotion");
    assert_eq!(
        runner.app().puzzles[0].description.as_deref(),
        Some("large-read-only-preface".repeat(20_000).as_str())
    );
}

#[test]
fn parity_layout_report_for_task_09() {
    let libra_h2o = DisplayMetrics {
        width: 1080,
        height: 1440,
        pixels_per_inch: 265,
        ..DisplayMetrics::default()
    };
    let simulator = DisplayMetrics {
        width: 1264,
        height: 1680,
        pixels_per_inch: 300,
        ..DisplayMetrics::default()
    };
    for (name, raw) in [
        ("kobo-legacy", EXAMPLE_PUZZLES),
        ("kindle-v1", KINDLE_V1),
        ("kindle-rich", RICH_PUZZLES),
        ("kindle-book", KINDLE_BOOK),
    ] {
        let (mut runner, _) = runner(Some(raw.to_vec()));
        let mut states = vec!["board".to_owned()];
        assert_layout_report(&runner, name, "board", &[libra_h2o, simulator]);
        runner.action(action_id(ANALYSIS_TOGGLE));
        assert!(runner.app().analysis_browser.is_open());
        let page_count = runner.app().analysis_pages().len();
        for destination in [0, page_count / 2, page_count - 1] {
            while runner.app().analysis_browser.page() < destination {
                runner.action(action_id(ANALYSIS_NEXT));
            }
            while runner.app().analysis_browser.page() > destination {
                runner.action(action_id(ANALYSIS_PREVIOUS));
            }
            let state = format!("analysis-{destination}");
            assert_layout_report(&runner, name, &state, &[libra_h2o, simulator]);
            states.push(state);
        }
        runner.action(action_id(ANALYSIS_CLOSE));
        runner.action(action_id(PUZZLE_PICKER));
        assert_layout_report(&runner, name, "picker", &[libra_h2o, simulator]);
        runner.action(action_id(PUZZLE_PICKER_CANCEL));
        runner.action(action_id(MODE_TOGGLE));
        assert_layout_report(&runner, name, "free-board", &[libra_h2o, simulator]);
        eprintln!("PARITY_LAYOUT checked={name} states={states:?},picker,free-board");
    }
}

fn assert_layout_report(
    runner: &AppRunner<ChessBoardApp>,
    fixture: &str,
    state: &str,
    metrics: &[DisplayMetrics],
) {
    let screen = runner.app().screen();
    for &size in metrics {
        let diagnostics = screen.diagnostics(&size, &Chrome::measuring(false));
        assert!(
            !diagnostics.has_errors(),
            "{fixture} {state} {size:?}: {:?}",
            diagnostics.issues
        );
        let board = diagnostics
            .layout
            .nodes
            .iter()
            .find(|node| matches!(node.kind, kobo_sdk::LayoutKind::ChessFrame { .. }))
            .expect("full-size board on every screen")
            .rect;
        for name in [MODE_TOGGLE, ORIENTATION_LOCK, RESET, FLIP, PUZZLE_PICKER] {
            assert!(diagnostics.layout.nodes.iter().any(
                |node| node.kind.acts_on() == Some(action_id(name))
            ));
        }
        for forbidden in ["description-toggle", "note-toggle", "game-review", "settings"] {
            assert!(!diagnostics.layout.nodes.iter().any(
                |node| node.kind.acts_on() == Some(action_id(forbidden))
            ));
        }
        eprintln!(
            "PARITY_LAYOUT fixture={fixture} state={state} display={}x{} board={board:?} issues=0",
            size.width, size.height
        );
    }
}
