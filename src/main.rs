use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

const BASE_URL: &str = "https://icmai.in/upload/";

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
    /// printf-style fmt: first %s = paper num, second %s = set index
    fmt: &'static str,
}

#[derive(Default)]
struct Stats {
    success: u32,
    skipped: u32,
    failed: u32,
}

// ---------------------------------------------------------------------------
// Static data tables
// ---------------------------------------------------------------------------

const PYQ_TERMS: &[PyqTerm] = &[
    PyqTerm { term: "Dec25",  prefix: "d25" },
    PyqTerm { term: "June25", prefix: "j25" },
    PyqTerm { term: "Dec24",  prefix: "d24" },
    PyqTerm { term: "Jun24",  prefix: "j24" },
    PyqTerm { term: "Dec23",  prefix: "d23" },
];

const MQP_Q_CONFIGS: &[MqpQConfig] = &[
    MqpQConfig { tag: "d25", path: "MQP_2022_Dec2025/Intermediate/",  sets: 2, fmt: "Paper{p}_Syl22_Dec25_Set{s}.pdf"  },
    MqpQConfig { tag: "j25", path: "MQP_2022_June2025/Intermediate/", sets: 2, fmt: "Paper{p}_Syl22_June25_Set{s}.pdf" },
    MqpQConfig { tag: "d24", path: "MQP_2022/Inter/",                 sets: 2, fmt: "MQP_Paper{p}_Set{s}_Dec24.pdf"    },
    MqpQConfig { tag: "j24", path: "MQP_2022/Inter/",                 sets: 1, fmt: "Paper{p}_Syl22_June24_Set1.pdf"   },
    MqpQConfig { tag: "d23", path: "MQP_2022/Inter/",                 sets: 2, fmt: "Paper{p}_Syl22_Dec23_Set{s}.pdf"  },
];

/// Apply {p} → paper, {s} → set in a format string.
fn fmt_url(template: &str, paper: &str, set: u8) -> String {
    template
        .replace("{p}", paper)
        .replace("{s}", &set.to_string())
}

// ---------------------------------------------------------------------------
// Per-paper question URL overrides (generative pattern 404s for these)
// ---------------------------------------------------------------------------

fn q_overrides() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    let mut m: HashMap<&str, HashMap<&str, &str>> = HashMap::new();

    m.insert("6", HashMap::from([
        ("pyq_j24_p6.pdf",    "QuestionPaper/syllabus2022/Jun24/P6_FA.pdf"),
        ("mqp_d24_s1_p6.pdf", "Students/MQP_2022/Inter/Paper6_Syl22_Dec24_Set1.pdf"),
        ("mqp_d24_s2_p6.pdf", "Students/MQP_2022_Dec2024/Inter/Q_MQP_Paper6_Set2_Dec24.pdf"),
    ]));
    m.insert("7", HashMap::from([
        ("mqp_d24_s2_p7.pdf", "Students/MQP_2022/Inter/Q_MQP_Paper7_Set2_Dec24.pdf"),
    ]));
    m.insert("8", HashMap::from([
        ("mqp_j25_s2_p8.pdf", "Students/MQP_2022_June2025/Intermediate/MQP_Paper8_Syl22_June2025_Set2.pdf"),
        ("mqp_d24_s1_p8.pdf", "Students/MQP_2022/Inter/Paper8_Syl22_Dec24_Set1.pdf"),
        ("mqp_d24_s2_p8.pdf", "Students/MQP_2022/Inter/Paper8_Syl22_Dec24_Set2.pdf"),
    ]));

    m
}

// ---------------------------------------------------------------------------
// MQP answer URLs — explicit per-paper (ICMAI naming too inconsistent to gen)
// ---------------------------------------------------------------------------

fn answer_urls() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    let mut m: HashMap<&str, HashMap<&str, &str>> = HashMap::new();

    m.insert("5", HashMap::from([
        ("mqp_ans_d25_s1_p5.pdf", "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper5_Dec25_Syl22.pdf"),
        ("mqp_ans_d25_s2_p5.pdf", "Students/MQP_2022_Dec2025/Intermediate/Ans_Paper5_Syl22_Dec25_Set2.pdf"),
        ("mqp_ans_j25_s1_p5.pdf", "Students/MQP_2022_June2025/Intermediate/MQP_Paper5_Set1_Jun25_Answer.pdf"),
        ("mqp_ans_j25_s2_p5.pdf", "Students/MQP_2022_June2025/Intermediate/Paper5_Syl22_June25_Set2_Sol.pdf"),
        ("mqp_ans_d24_s1_p5.pdf", "Students/MQP_2022/Inter/A_MQP_Paper5_Set1_Dec24.pdf"),
        ("mqp_ans_d24_s2_p5.pdf", "Students/MQP_2022/Inter/A_MQP_Paper5_Set2_Dec24.pdf"),
        ("mqp_ans_j24_s1_p5.pdf", "Students/MQP_2022/Inter/Paper5_Syl22_June24_Set1_Sol.pdf"),
        ("mqp_ans_d23_s1_p5.pdf", "Students/MQP_2022/Inter/Paper5_Syl22_Dec23_Set1_Sol.pdf"),
        ("mqp_ans_d23_s2_p5.pdf", "Students/MQP_2022/Inter/Paper5_Syl22_Dec23_Set2_Sol.pdf"),
    ]));
    m.insert("6", HashMap::from([
        ("mqp_ans_d25_s1_p6.pdf", "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper6_Dec25_Syl22.pdf"),
        ("mqp_ans_d25_s2_p6.pdf", "Students/MQP_2022_Dec2025/Intermediate/Ans_Paper6_Syl22_Dec25_Set2.pdf"),
        ("mqp_ans_j25_s1_p6.pdf", "Students/MQP_2022_June2025/Intermediate/Paper6_Syl22_June25_Set1_Sol.pdf"),
        ("mqp_ans_j25_s2_p6.pdf", "Students/MQP_2022_June2025/Intermediate/Paper6_Syl22_June25_Set2_Sol.pdf"),
        ("mqp_ans_d24_s1_p6.pdf", "Students/MQP_2022/Inter/A_MQP_Paper6_Set1_Dec24.pdf"),
        ("mqp_ans_d24_s2_p6.pdf", "Students/MQP_2022/Inter/A_MQP_Paper6_Set2_Dec24.pdf"),
        ("mqp_ans_j24_s1_p6.pdf", "Students/MQP_2022/Inter/Paper6_Syl22_June24_Set1_Sol.pdf"),
        ("mqp_ans_d23_s1_p6.pdf", "Students/MQP_2022/Inter/Paper6_Syl22_Dec23_Set1_Sol.pdf"),
        ("mqp_ans_d23_s2_p6.pdf", "Students/MQP_2022/Inter/Paper6_Syl22_Dec23_Set2_Sol.pdf"),
    ]));
    m.insert("7", HashMap::from([
        ("mqp_ans_d25_s1_p7.pdf", "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper7_Dec25_Syl22.pdf"),
        // d25 s2: 404 confirmed
        ("mqp_ans_j25_s1_p7.pdf", "Students/MQP_2022_June2025/Intermediate/Paper7_Syl22_June25_Set1_Sol.pdf"),
        ("mqp_ans_j25_s2_p7.pdf", "Students/MQP_2022_June2025/Intermediate/Paper7_Syl22_June25_Set2_Sol.pdf"),
        ("mqp_ans_d24_s1_p7.pdf", "Students/MQP_2022/Inter/A_MQP_Paper7_Set1_Dec24.pdf"),
        ("mqp_ans_d24_s2_p7.pdf", "Students/MQP_2022/Inter/A_MQP_Paper7_Set2_Dec24.pdf"),
        ("mqp_ans_j24_s1_p7.pdf", "Students/MQP_2022/Inter/Paper7_Syl22_June24_Set1_Sol.pdf"),
        ("mqp_ans_d23_s1_p7.pdf", "Students/MQP_2022/Inter/Paper7_Syl22_Dec23_Set1_Sol.pdf"),
        ("mqp_ans_d23_s2_p7.pdf", "Students/MQP_2022/Inter/Paper7_Syl22_Dec23_Set2_Sol.pdf"),
    ]));
    m.insert("8", HashMap::from([
        ("mqp_ans_d25_s1_p8.pdf", "Students/MQP_2022_Dec2025/Intermediate/AnswersMQP_Set1_Paper8_Dec25_Syl22.pdf"),
        ("mqp_ans_d25_s2_p8.pdf", "Students/MQP_2022_Dec2025/Intermediate/Q&A_MQP_Paper8_Syllabus22_Dec2025_Set2.pdf"),
        ("mqp_ans_j25_s1_p8.pdf", "Students/MQP_2022_June2025/Intermediate/A_MQP_Paper8_Set1_Jun25.pdf"),
        ("mqp_ans_j25_s2_p8.pdf", "Students/MQP_2022_June2025/Intermediate/A_MQP_Paper8_Syl22_June2025_Set2.pdf"),
        ("mqp_ans_d24_s1_p8.pdf", "Students/MQP_2022/Inter/A_MQP_Paper8_Set1_Dec24.pdf"),
        ("mqp_ans_d24_s2_p8.pdf", "Students/MQP_2022/Inter/A_MQP_Paper8_Set2_Dec24.pdf"),
        ("mqp_ans_j24_s1_p8.pdf", "Students/MQP_2022/Inter/Paper8_Syl22_June24_Set1_Sol.pdf"),
        ("mqp_ans_d23_s1_p8.pdf", "Students/MQP_2022/Inter/Paper8_Syl22_Dec23_Set1_Sol.pdf"),
        ("mqp_ans_d23_s2_p8.pdf", "Students/MQP_2022/Inter/Paper8_Syl22_Dec23_Set2_Sol.pdf"),
    ]));
    // Add "9" => { ... } etc. once verified

    m
}

// ---------------------------------------------------------------------------
// Download logic
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

fn record(stats: &mut Stats, total: &mut Stats, result: DownloadResult, filename: &str) {
    match result {
        DownloadResult::Saved => {
            stats.success += 1;
            total.success += 1;
            println!(" [+] Saved:   {filename}");
        }
        DownloadResult::Skipped => {
            stats.skipped += 1;
            total.skipped += 1;
            println!(" [~] Skipped: {filename}");
        }
        DownloadResult::Missing => {
            stats.failed += 1;
            total.failed += 1;
            println!(" [-] Missing: {filename}");
        }
    }
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

fn parse_papers(input: &str) -> Result<Vec<String>, String> {
    if input == "all" {
        return Ok((5u32..=12).map(|n| n.to_string()).collect());
    }
    if let Some((a, b)) = input.split_once('-') {
        let start: u32 = a.parse().map_err(|_| format!("invalid range start: {a}"))?;
        let end: u32   = b.parse().map_err(|_| format!("invalid range end: {b}"))?;
        if start > end {
            return Err(format!("range {start}-{end} is empty"));
        }
        return Ok((start..=end).map(|n| n.to_string()).collect());
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
    let arg = env::args().nth(1).unwrap_or_else(|| "5".to_string());
    let papers = parse_papers(&arg).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!("Usage: qps [<paper_num> | <start>-<end> | all]");
        std::process::exit(1);
    });

    let overrides   = q_overrides();
    let answers     = answer_urls();
    let mut total   = Stats::default();
    let sep         = "-".repeat(32);
    let sep_wide    = "=".repeat(32);

    for num in &papers {
        let folder = format!("p{num}");
        fs::create_dir_all(&folder).unwrap_or_else(|e| {
            eprintln!("failed to create {folder}: {e}");
            std::process::exit(1);
        });

        let mut stats = Stats::default();
        let paper_overrides = overrides.get(num.as_str()).cloned().unwrap_or_default();

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
            record(&mut stats, &mut total, result, &filename);
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
                record(&mut stats, &mut total, result, &filename);
            }
        }

        // MQP answers
        match answers.get(num.as_str()) {
            Some(paper_answers) => {
                let mut sorted: Vec<_> = paper_answers.iter().collect();
                sorted.sort_by_key(|(f, _)| *f);
                for (filename, rel_url) in sorted {
                    let url = format!("{BASE_URL}{rel_url}");
                    let result = download(&folder, filename, &url);
                    record(&mut stats, &mut total, result, filename);
                }
            }
            None => {
                println!(" [!] No answer URLs defined for paper {num} — add them to answer_urls()");
            }
        }

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

    if total.failed > 0 {
        std::process::exit(1);
    }
}
