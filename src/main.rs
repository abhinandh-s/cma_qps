use std::collections::HashMap;
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

const BASE_URL: &str = "https://icmai.in/upload/";

// ---------------------------------------------------------------------------
// Paper metadata
// ---------------------------------------------------------------------------

struct Paper {
    num: &'static str,
    name: &'static str,
}

const PAPERS: &[Paper] = &[
    Paper {
        num: "5",
        name: "Law and Ethics",
    },
    Paper {
        num: "6",
        name: "Financial Accounting",
    },
    Paper {
        num: "7",
        name: "Direct and Indirect Taxation",
    },
    Paper {
        num: "8",
        name: "Cost Accounting",
    },
    Paper {
        num: "9",
        name: "Operations Management and Strategic Management",
    },
    Paper {
        num: "10",
        name: "Corporate Accounting and Auditing",
    },
    Paper {
        num: "11",
        name: "Financial Management and Business Data Analytics",
    },
    Paper {
        num: "12",
        name: "Management Accounting",
    },
];

fn paper_name(num: &str) -> &'static str {
    PAPERS
        .iter()
        .find(|p| p.num == num)
        .map(|p| p.name)
        .unwrap_or("Unknown")
}

// ---------------------------------------------------------------------------
// Session display labels (tag → human label, sort key)
// ---------------------------------------------------------------------------

struct Session {
    tag: &'static str,
    label: &'static str,
    sort_key: u32, // higher = more recent
}

const SESSIONS: &[Session] = &[
    Session {
        tag: "d25",
        label: "Dec 2025",
        sort_key: 50,
    },
    Session {
        tag: "j25",
        label: "Jun 2025",
        sort_key: 49,
    },
    Session {
        tag: "d24",
        label: "Dec 2024",
        sort_key: 48,
    },
    Session {
        tag: "j24",
        label: "Jun 2024",
        sort_key: 47,
    },
    Session {
        tag: "d23",
        label: "Dec 2023",
        sort_key: 46,
    },
];

fn session_label(tag: &str) -> &str {
    SESSIONS
        .iter()
        .find(|s| s.tag == tag)
        .map(|s| s.label)
        .unwrap_or(tag)
}

fn session_sort_key(tag: &str) -> u32 {
    SESSIONS
        .iter()
        .find(|s| s.tag == tag)
        .map(|s| s.sort_key)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

struct PyqTerm {
    term: &'static str,
    prefix: &'static str,
}

struct MqpQConfig {
    tag: &'static str,
    path: &'static str,
    sets: u8,
    fmt: &'static str,
}

#[derive(Default)]
struct Stats {
    success: u32,
    skipped: u32,
    failed: u32,
}

/// A successfully downloaded file — stored for release note generation.
struct Downloaded {
    filename: String,
    /// tag extracted from filename (d25, j25, etc.)
    session: String,
    /// set number if MQP, else 0
    #[allow(unused)]
    set: u8,
}

// ---------------------------------------------------------------------------
// Static data tables
// ---------------------------------------------------------------------------

const PYQ_TERMS: &[PyqTerm] = &[
    PyqTerm {
        term: "Dec25",
        prefix: "d25",
    },
    PyqTerm {
        term: "June25",
        prefix: "j25",
    },
    PyqTerm {
        term: "Dec24",
        prefix: "d24",
    },
    PyqTerm {
        term: "Jun24",
        prefix: "j24",
    },
    PyqTerm {
        term: "Dec23",
        prefix: "d23",
    },
];

const MQP_Q_CONFIGS: &[MqpQConfig] = &[
    MqpQConfig {
        tag: "d25",
        path: "MQP_2022_Dec2025/Intermediate/",
        sets: 2,
        fmt: "Paper{p}_Syl22_Dec25_Set{s}.pdf",
    },
    MqpQConfig {
        tag: "j25",
        path: "MQP_2022_June2025/Intermediate/",
        sets: 2,
        fmt: "Paper{p}_Syl22_June25_Set{s}.pdf",
    },
    MqpQConfig {
        tag: "d24",
        path: "MQP_2022/Inter/",
        sets: 2,
        fmt: "MQP_Paper{p}_Set{s}_Dec24.pdf",
    },
    MqpQConfig {
        tag: "j24",
        path: "MQP_2022/Inter/",
        sets: 1,
        fmt: "Paper{p}_Syl22_June24_Set1.pdf",
    },
    MqpQConfig {
        tag: "d23",
        path: "MQP_2022/Inter/",
        sets: 2,
        fmt: "Paper{p}_Syl22_Dec23_Set{s}.pdf",
    },
];

fn fmt_url(template: &str, paper: &str, set: u8) -> String {
    template
        .replace("{p}", paper)
        .replace("{s}", &set.to_string())
}

// ---------------------------------------------------------------------------
// Per-paper question URL overrides
// ---------------------------------------------------------------------------

fn q_overrides() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    let mut m: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    m.insert(
        "6",
        HashMap::from([
            (
                "pyq_j24_p6.pdf",
                "QuestionPaper/syllabus2022/Jun24/P6_FA.pdf",
            ),
            (
                "mqp_d24_s1_p6.pdf",
                "Students/MQP_2022/Inter/Paper6_Syl22_Dec24_Set1.pdf",
            ),
            (
                "mqp_d24_s2_p6.pdf",
                "Students/MQP_2022_Dec2024/Inter/Q_MQP_Paper6_Set2_Dec24.pdf",
            ),
        ]),
    );
    m.insert(
        "7",
        HashMap::from([(
            "mqp_d24_s2_p7.pdf",
            "Students/MQP_2022/Inter/Q_MQP_Paper7_Set2_Dec24.pdf",
        )]),
    );
    m.insert(
        "8",
        HashMap::from([
            (
                "mqp_j25_s2_p8.pdf",
                "Students/MQP_2022_June2025/Intermediate/MQP_Paper8_Syl22_June2025_Set2.pdf",
            ),
            (
                "mqp_d24_s1_p8.pdf",
                "Students/MQP_2022/Inter/Paper8_Syl22_Dec24_Set1.pdf",
            ),
            (
                "mqp_d24_s2_p8.pdf",
                "Students/MQP_2022/Inter/Paper8_Syl22_Dec24_Set2.pdf",
            ),
        ]),
    );
    m.insert(
        "9",
        HashMap::from([
            (
                "pyq_j24_p9.pdf",
                "/QuestionPaper/syllabus2022/Jun24/P9_OMSM.pdf",
            ),
            (
                "mqp_d24_s1_p9.pdf",
                "Students/MQP_2022/Inter/MQP_Paper9_Set1_Dec24_R.pdf",
            ),
            (
                "mqp_d24_s2_p9.pdf",
                "Students/MQP_2022/Inter/Paper9_Syl22_Dec24_Set2.pdf",
            ),
        ]),
    );
    m.insert(
        "10",
        HashMap::from([
            (
                "pyq_j24_p10.pdf",
                "QuestionPaper/syllabus2022/Jun24/P10_CAA.pdf",
            ),
            (
                "mqp_d25_s2_p10.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/MQP_Paper10_syllabus22_Dec2025_Set2.pdf",
            ),
        ]),
    );

    m.insert(
        "11",
        HashMap::from([
            (
                "mqp_d25_s2_p11.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/MQP_Paper11_Set2_December2025.pdf",
            ),
            (
                "mqp_j25_s2_p11.pdf",
                "Students/MQP_2022_June2025/Intermediate/MQP_Paper11_Syl22_June2025_Set2.pdf",
            ),
            (
                "mqp_d24_s1_p11.pdf",
                "Students/MQP_2022/Inter/Paper11_Syl22_Dec24_Set1.pdf",
            ),
        ]),
    );
    m.insert(
        "12",
        HashMap::from([
            (
                "mqp_d25_s2_p12.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/MQP_Paper12_syllabus2022_Dec25_Set_2.pdf",
            ),
            (
                "mqp_j25_s2_p12.pdf",
                "Students/MQP_2022_June2025/Intermediate/MQP_Paper12_Syl22_June2025_Set2.pdf",
            ),
            (
                "mqp_d24_s2_p12.pdf",
                "Students/MQP_2022/Inter/Q_MQP_Paper12_Set2_Dec24.pdf",
            ),
        ]),
    );
    m
}

// ---------------------------------------------------------------------------
// MQP answer URLs
// ---------------------------------------------------------------------------

fn answer_urls() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    let mut m: HashMap<&str, HashMap<&str, &str>> = HashMap::new();
    m.insert(
        "5",
        HashMap::from([
            (
                "mqp_ans_d25_s1_p5.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper5_Dec25_Syl22.pdf",
            ),
            (
                "mqp_ans_d25_s2_p5.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/Ans_Paper5_Syl22_Dec25_Set2.pdf",
            ),
            (
                "mqp_ans_j25_s1_p5.pdf",
                "Students/MQP_2022_June2025/Intermediate/MQP_Paper5_Set1_Jun25_Answer.pdf",
            ),
            (
                "mqp_ans_j25_s2_p5.pdf",
                "Students/MQP_2022_June2025/Intermediate/Paper5_Syl22_June25_Set2_Sol.pdf",
            ),
            (
                "mqp_ans_d24_s1_p5.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper5_Set1_Dec24.pdf",
            ),
            (
                "mqp_ans_d24_s2_p5.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper5_Set2_Dec24.pdf",
            ),
            (
                "mqp_ans_j24_s1_p5.pdf",
                "Students/MQP_2022/Inter/Paper5_Syl22_June24_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s1_p5.pdf",
                "Students/MQP_2022/Inter/Paper5_Syl22_Dec23_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s2_p5.pdf",
                "Students/MQP_2022/Inter/Paper5_Syl22_Dec23_Set2_Sol.pdf",
            ),
        ]),
    );
    m.insert(
        "6",
        HashMap::from([
            (
                "mqp_ans_d25_s1_p6.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper6_Dec25_Syl22.pdf",
            ),
            (
                "mqp_ans_d25_s2_p6.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/Ans_Paper6_Syl22_Dec25_Set2.pdf",
            ),
            (
                "mqp_ans_j25_s1_p6.pdf",
                "Students/MQP_2022_June2025/Intermediate/Paper6_Syl22_June25_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_j25_s2_p6.pdf",
                "Students/MQP_2022_June2025/Intermediate/Paper6_Syl22_June25_Set2_Sol.pdf",
            ),
            (
                "mqp_ans_d24_s1_p6.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper6_Set1_Dec24.pdf",
            ),
            (
                "mqp_ans_d24_s2_p6.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper6_Set2_Dec24.pdf",
            ),
            (
                "mqp_ans_j24_s1_p6.pdf",
                "Students/MQP_2022/Inter/Paper6_Syl22_June24_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s1_p6.pdf",
                "Students/MQP_2022/Inter/Paper6_Syl22_Dec23_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s2_p6.pdf",
                "Students/MQP_2022/Inter/Paper6_Syl22_Dec23_Set2_Sol.pdf",
            ),
        ]),
    );
    m.insert(
        "7",
        HashMap::from([
            (
                "mqp_ans_d25_s1_p7.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper7_Dec25_Syl22.pdf",
            ),
            (
                "mqp_ans_j25_s1_p7.pdf",
                "Students/MQP_2022_June2025/Intermediate/Paper7_Syl22_June25_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_j25_s2_p7.pdf",
                "Students/MQP_2022_June2025/Intermediate/Paper7_Syl22_June25_Set2_Sol.pdf",
            ),
            (
                "mqp_ans_d24_s1_p7.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper7_Set1_Dec24.pdf",
            ),
            (
                "mqp_ans_d24_s2_p7.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper7_Set2_Dec24.pdf",
            ),
            (
                "mqp_ans_j24_s1_p7.pdf",
                "Students/MQP_2022/Inter/Paper7_Syl22_June24_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s1_p7.pdf",
                "Students/MQP_2022/Inter/Paper7_Syl22_Dec23_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s2_p7.pdf",
                "Students/MQP_2022/Inter/Paper7_Syl22_Dec23_Set2_Sol.pdf",
            ),
        ]),
    );
    m.insert(
        "8",
        HashMap::from([
            (
                "mqp_ans_d25_s1_p8.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper8_Dec25_Syl22.pdf",
            ),
            (
                "mqp_ans_d25_s2_p8.pdf",
                "Students/MQP_2022_Dec2025/Intermediate/Q&A_MQP_Paper8_Syllabus22_Dec2025_Set2.pdf",
            ),
            (
                "mqp_ans_j25_s1_p8.pdf",
                "Students/MQP_2022_June2025/Intermediate/A_MQP_Paper8_Set1_Jun25.pdf",
            ),
            (
                "mqp_ans_j25_s2_p8.pdf",
                "Students/MQP_2022_June2025/Intermediate/A_MQP_Paper8_Syl22_June2025_Set2.pdf",
            ),
            (
                "mqp_ans_d24_s1_p8.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper8_Set1_Dec24.pdf",
            ),
            (
                "mqp_ans_d24_s2_p8.pdf",
                "Students/MQP_2022/Inter/A_MQP_Paper8_Set2_Dec24.pdf",
            ),
            (
                "mqp_ans_j24_s1_p8.pdf",
                "Students/MQP_2022/Inter/Paper8_Syl22_June24_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s1_p8.pdf",
                "Students/MQP_2022/Inter/Paper8_Syl22_Dec23_Set1_Sol.pdf",
            ),
            (
                "mqp_ans_d23_s2_p8.pdf",
                "Students/MQP_2022/Inter/Paper8_Syl22_Dec23_Set2_Sol.pdf",
            ),
        ]),
    );
    m
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

enum DownloadResult {
    Saved,
    Skipped,
    Missing,
}

fn download(folder: &str, filename: &str, url: &str) -> DownloadResult {
    let path = format!("{}/{}", folder, filename);
    if Path::new(&path).exists() {
        return DownloadResult::Skipped;
    }
    let resp = reqwest::blocking::get(url);
    match resp {
        Ok(r) if r.status().is_success() => {
            let bytes = r.bytes().unwrap_or_default();
            match fs::write(&path, &bytes) {
                Ok(_) => DownloadResult::Saved,
                Err(e) => {
                    eprintln!("     write error {path}: {e}");
                    let _ = fs::remove_file(&path);
                    DownloadResult::Missing
                }
            }
        }
        _ => {
            let _ = fs::remove_file(&path);
            DownloadResult::Missing
        }
    }
}

fn record(
    stats: &mut Stats,
    total: &mut Stats,
    result: DownloadResult,
    filename: &str,
    downloaded: &mut Vec<Downloaded>,
    session: &str,
    set: u8,
) {
    match result {
        DownloadResult::Saved => {
            stats.success += 1;
            total.success += 1;
            println!(" [+] Saved:   {filename}");
            downloaded.push(Downloaded {
                filename: filename.to_string(),
                session: session.to_string(),
                set,
            });
        }
        DownloadResult::Skipped => {
            stats.skipped += 1;
            total.skipped += 1;
            println!(" [~] Skipped: {filename}");
            downloaded.push(Downloaded {
                filename: filename.to_string(),
                session: session.to_string(),
                set,
            });
        }
        DownloadResult::Missing => {
            stats.failed += 1;
            total.failed += 1;
            println!(" [-] Missing: {filename}");
        }
    }
}

// ---------------------------------------------------------------------------
// Release notes generation
// ---------------------------------------------------------------------------

/// GitHub release asset download URL base.
/// CI passes this via --release-base <url>, e.g.
/// https://github.com/abhinandh-s/cma_resources/releases/download/2026-03
fn asset_url(base: &str, filename: &str) -> String {
    // GitHub encodes spaces but filenames here are safe
    format!("{}/{}", base.trim_end_matches('/'), filename)
}

/// Parse session tag + set out of a filename like:
///   pyq_d25_p5.pdf        → ("d25", 0, "pyq")
///   mqp_j24_s1_p8.pdf     → ("j24", 1, "mqp")
///   mqp_ans_d23_s2_p7.pdf → ("d23", 2, "mqp_ans")
fn parse_filename(filename: &str) -> Option<(&str, u8, &str)> {
    // Strip .pdf
    let stem = filename.strip_suffix(".pdf")?;
    let parts: Vec<&str> = stem.splitn(6, '_').collect();
    // pyq_<tag>_p<n>
    if parts[0] == "pyq" && parts.len() == 3 {
        return Some((parts[1], 0, "pyq"));
    }
    // mqp_<tag>_s<n>_p<n>
    if parts[0] == "mqp" && parts.get(1).copied() != Some("ans") && parts.len() == 4 {
        let set = parts[2].trim_start_matches('s').parse().ok()?;
        return Some((parts[1], set, "mqp"));
    }
    // mqp_ans_<tag>_s<n>_p<n>
    if parts[0] == "mqp" && parts.get(1).copied() == Some("ans") && parts.len() == 5 {
        let set = parts[3].trim_start_matches('s').parse().ok()?;
        return Some((parts[2], set, "mqp_ans"));
    }
    None
}

fn generate_release_notes(
    per_paper: &HashMap<String, Vec<Downloaded>>,
    release_base: &str,
    total: &Stats,
) -> String {
    let mut md = String::new();

    // GitHub release title is set separately — no h1 here
    writeln!(
        md,
        "> **{} files** across {} papers. \
         Files not listed here were not yet uploaded by ICMAI at time of release.",
        total.success + total.skipped,
        per_paper.len()
    )
    .unwrap();
    writeln!(md).unwrap();

    let mut paper_nums: Vec<&String> = per_paper.keys().collect();
    paper_nums.sort_by_key(|n| n.parse::<u32>().unwrap_or(99));

    for num in paper_nums {
        let name = paper_name(num);
        writeln!(md, "## P{num} — {name}").unwrap();
        writeln!(md).unwrap();

        let files = &per_paper[num];

        // ---- PYQ ----
        let mut pyqs: Vec<&Downloaded> = files
            .iter()
            .filter(|f| {
                parse_filename(&f.filename)
                    .map(|(_, _, k)| k == "pyq")
                    .unwrap_or(false)
            })
            .collect();
        pyqs.sort_by_key(|f| std::cmp::Reverse(session_sort_key(&f.session)));

        if !pyqs.is_empty() {
            writeln!(md, "### Previous Year Question Papers").unwrap();
            writeln!(md).unwrap();
            for f in &pyqs {
                let label = session_label(&f.session);
                let url = asset_url(release_base, &f.filename);
                writeln!(md, "- [{label}]({url})").unwrap();
            }
            writeln!(md).unwrap();
        }

        // ---- MQP: collect unique sessions that have question files ----
        let mut mqp_session_set: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for f in files {
            if let Some((tag, _, "mqp")) = parse_filename(&f.filename) {
                mqp_session_set.insert(tag);
            }
        }
        let mut mqp_sessions: Vec<&str> = mqp_session_set.into_iter().collect();
        mqp_sessions.sort_by_key(|t| std::cmp::Reverse(session_sort_key(t)));

        if !mqp_sessions.is_empty() {
            writeln!(md, "### Model Question Papers").unwrap();
            writeln!(md).unwrap();

            for session_tag in &mqp_sessions {
                writeln!(md, "#### {}", session_label(session_tag)).unwrap();
                writeln!(md).unwrap();

                // collect set numbers for this session's question files
                let mut set_nums: std::collections::HashSet<u8> = std::collections::HashSet::new();
                for f in files {
                    if let Some((t, s, "mqp")) = parse_filename(&f.filename)
                        && t == *session_tag
                    {
                        set_nums.insert(s);
                    }
                }
                // also include sets that only have an answer (no question file)
                for f in files {
                    if let Some((t, s, "mqp_ans")) = parse_filename(&f.filename)
                        && t == *session_tag
                    {
                        set_nums.insert(s);
                    }
                }
                let mut sets: Vec<u8> = set_nums.into_iter().collect();
                sets.sort();

                for set in sets {
                    writeln!(md, "**Set {set}**").unwrap();
                    writeln!(md).unwrap();

                    let q = files.iter().find(|f| {
                        parse_filename(&f.filename)
                            .map(|(t, s, k)| t == *session_tag && s == set && k == "mqp")
                            .unwrap_or(false)
                    });
                    let a = files.iter().find(|f| {
                        parse_filename(&f.filename)
                            .map(|(t, s, k)| t == *session_tag && s == set && k == "mqp_ans")
                            .unwrap_or(false)
                    });

                    match (q, a) {
                        (Some(q), Some(a)) => {
                            writeln!(
                                md,
                                "| [<picture>
<source media=\"(prefers-color-scheme: dark)\"
srcset=\"https://raw.githubusercontent.com/abhinandh-s/cma_qps/refs/heads/master/assets/file-unknown-dark.svg\">
<img src=\"https://raw.githubusercontent.com/abhinandh-s/cma_qps/refs/heads/master/assets/file-unknown-light.svg\" width=\"24\">
</picture> Question Paper]({}) | [󱪚 Answer Key]({}) |",
                                asset_url(release_base, &q.filename),
                                asset_url(release_base, &a.filename),
                            )
                            .unwrap();
                            writeln!(md, "|---|---|").unwrap();
                        }
                        (Some(q), None) => {
                            writeln!(
                                md,
                                "- [<picture>
<source media=\"(prefers-color-scheme: dark)\"
srcset=\"https://raw.githubusercontent.com/abhinandh-s/cma_qps/refs/heads/master/assets/file-unknown-dark.svg\">
<img src=\"https://raw.githubusercontent.com/abhinandh-s/cma_qps/refs/heads/master/assets/file-unknown-light.svg\" width=\"24\">
</picture> Question Paper]({})",
                                asset_url(release_base, &q.filename)
                            )
                            .unwrap();
                            writeln!(md, "- ~~Answer Key~~ *(not uploaded by ICMAI)*").unwrap();
                        }
                        (None, Some(a)) => {
                            writeln!(md, "- ~~Question Paper~~ *(not uploaded by ICMAI)*").unwrap();
                            writeln!(
                                md,
                                "- [󱪚 Answer Key]({})",
                                asset_url(release_base, &a.filename)
                            )
                            .unwrap();
                        }
                        (None, None) => {}
                    }
                    writeln!(md).unwrap();
                }
            }
        }

        writeln!(md, "---").unwrap();
        writeln!(md).unwrap();
    }

    md
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

struct Args {
    papers: Vec<String>,
    release_base: String,
    tag: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1).peekable();

    let mut paper_input = String::from("5");
    let mut release_base = String::new();
    let mut tag = String::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--release-base" => {
                release_base = args.next().ok_or("--release-base requires a value")?;
            }
            "--tag" => {
                tag = args.next().ok_or("--tag requires a value")?;
            }
            other => paper_input = other.to_string(),
        }
    }

    let papers = parse_papers(&paper_input)?;
    Ok(Args {
        papers,
        release_base,
        tag,
    })
}

fn parse_papers(input: &str) -> Result<Vec<String>, String> {
    if input == "all" {
        return Ok((5u32..=12).map(|n| n.to_string()).collect());
    }
    if let Some((a, b)) = input.split_once('-') {
        // Disambiguate "5-8" (range) from a negative or other oddity
        if let (Ok(start), Ok(end)) = (a.parse::<u32>(), b.parse::<u32>()) {
            if start > end {
                return Err(format!("range {start}-{end} is empty"));
            }
            return Ok((start..=end).map(|n| n.to_string()).collect());
        }
    }
    if input.chars().all(|c| c.is_ascii_digit()) {
        return Ok(vec![input.to_string()]);
    }
    Err(format!("unrecognised input: {input}"))
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let Args {
        papers,
        release_base,
        tag,
    } = parse_args().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!(
            "Usage: qps [<paper_num>|<start>-<end>|all] [--release-base <url>] [--tag <tag>]"
        );
        std::process::exit(1);
    });

    let overrides = q_overrides();
    let answers = answer_urls();
    let mut total = Stats::default();
    let sep = "-".repeat(32);
    let sep_wide = "=".repeat(32);

    // paper_num → collected files (saved + skipped = present on disk)
    let mut per_paper: HashMap<String, Vec<Downloaded>> = HashMap::new();

    for num in &papers {
        let folder = format!("p{num}");
        fs::create_dir_all(&folder).unwrap_or_else(|e| {
            eprintln!("failed to create {folder}: {e}");
            std::process::exit(1);
        });

        let mut stats = Stats::default();
        let paper_overrides = overrides.get(num.as_str()).cloned().unwrap_or_default();
        let mut downloaded: Vec<Downloaded> = Vec::new();

        println!("\n--- Session: Paper {num} ---");

        // PYQs
        for t in PYQ_TERMS {
            let filename = format!("pyq_{}_p{}.pdf", t.prefix, num);
            let url = match paper_overrides.get(filename.as_str()) {
                Some(rel) => format!("{BASE_URL}{rel}"),
                None => format!(
                    "{BASE_URL}QuestionPaper/syllabus2022/{}/Paper{num}.pdf",
                    t.term
                ),
            };
            let result = download(&folder, &filename, &url);
            record(
                &mut stats,
                &mut total,
                result,
                &filename,
                &mut downloaded,
                t.prefix,
                0,
            );
        }

        // MQP questions
        for c in MQP_Q_CONFIGS {
            for s in 1..=c.sets {
                let filename = format!("mqp_{}_s{}_p{}.pdf", c.tag, s, num);
                let url = match paper_overrides.get(filename.as_str()) {
                    Some(rel) => format!("{BASE_URL}{rel}"),
                    None => {
                        let file_part = fmt_url(c.fmt, num, s);
                        format!("{BASE_URL}Students/{}{}", c.path, file_part)
                    }
                };
                let result = download(&folder, &filename, &url);
                record(
                    &mut stats,
                    &mut total,
                    result,
                    &filename,
                    &mut downloaded,
                    c.tag,
                    s,
                );
            }
        }

        // MQP answers
        match answers.get(num.as_str()) {
            Some(paper_answers) => {
                let mut sorted: Vec<_> = paper_answers.iter().collect();
                sorted.sort_by_key(|(f, _)| *f);
                for (filename, rel_url) in sorted {
                    let url = format!("{BASE_URL}{rel_url}");
                    // parse session + set from filename for record()
                    let (sess, set) = parse_filename(filename)
                        .map(|(t, s, _)| (t.to_string(), s))
                        .unwrap_or_default();
                    let result = download(&folder, filename, &url);
                    record(
                        &mut stats,
                        &mut total,
                        result,
                        filename,
                        &mut downloaded,
                        &sess,
                        set,
                    );
                }
            }
            None => {
                println!(" [!] No answer URLs defined for paper {num} — add them to answer_urls()");
            }
        }

        per_paper.insert(num.clone(), downloaded);

        println!("{sep}");
        println!(
            "Paper {:<3}  ↓ {:<3}  ~ {:<3}  ✗ {}",
            num, stats.success, stats.skipped, stats.failed
        );
        println!("{sep}");
    }

    if papers.len() > 1 {
        println!("\n{sep_wide}");
        println!(
            "TOTAL      ↓ {:<3}  ~ {:<3}  ✗ {}",
            total.success, total.skipped, total.failed
        );
        println!("{sep_wide}");
    }

    // Write release notes if --release-base was provided
    if !release_base.is_empty() {
        let _effective_tag = if tag.is_empty() {
            "latest".to_string()
        } else {
            tag
        };
        let notes = generate_release_notes(&per_paper, &release_base, &total);
        fs::write("release_notes.md", &notes).unwrap_or_else(|e| {
            eprintln!("failed to write release_notes.md: {e}");
        });
        println!("\nRelease notes written to release_notes.md");
    }

    if total.failed > 0 {
        std::process::exit(1);
    }
}
