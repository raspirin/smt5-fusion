use std::{collections::HashSet, hint::black_box, rc::Rc, time::Instant};

use smt5_fusion_core::{
    data::game_data::GameData,
    dataset::{self, demon_ids, skill_ids},
    model::{
        demon::{DemonId, SkillAcquisition},
        player_context::PlayerContext,
        route_space::{RouteChoice, RouteSelector, RouteSpace},
        skill::{SkillCategory, SkillId},
    },
    reverse_search::{SearchRequest, search},
};

const DEFAULT_WARMUPS: usize = 2;
const DEFAULT_SAMPLES: usize = 20;

struct Config {
    warmups: usize,
    samples: usize,
    reverse: bool,
}

#[derive(Default)]
struct GraphStats {
    spaces: usize,
    choices: usize,
    material_options: usize,
    edges: usize,
}

fn main() {
    let config = parse_args().unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    let setup_started = Instant::now();
    let data = dataset::game_data();
    let mut context = PlayerContext::default();
    context.set_konohana_sakuya_dlc(true);
    context.set_dagda_dlc(true);
    context.prepare_direct_recipes(&data);

    println!("# runner=android-core-runner");
    println!("# arch={}", std::env::consts::ARCH);
    println!("# os={}", std::env::consts::OS);
    println!("# warmups={}", config.warmups);
    println!("# samples={}", config.samples);
    println!(
        "# setup_ms={:.3}",
        setup_started.elapsed().as_secs_f64() * 1000.0
    );
    println!(
        "target,mode,median_ms,p95_ms,max_ms,spaces,choices,material_options,edges,count_digits,count"
    );

    let mut cases = benchmark_cases(&data);
    if config.reverse {
        cases.reverse();
    }
    for (name, mode, request) in cases {
        for _ in 0..config.warmups {
            black_box(search(&data, &context, &request).unwrap());
        }

        let selector = search(&data, &context, &request).unwrap();
        let stats = graph_stats(&selector);
        let route_count = selector.route_count.to_string();
        drop(selector);

        let mut samples = Vec::with_capacity(config.samples);
        for _ in 0..config.samples {
            let started = Instant::now();
            let selector = black_box(search(&data, &context, &request).unwrap());
            samples.push(started.elapsed());
            black_box(&selector);
            drop(selector);
        }
        samples.sort_unstable();

        let median = samples[config.samples / 2].as_secs_f64() * 1000.0;
        let p95_index = (config.samples * 95).div_ceil(100) - 1;
        let p95 = samples[p95_index].as_secs_f64() * 1000.0;
        let maximum = samples[config.samples - 1].as_secs_f64() * 1000.0;
        println!(
            "{name},{mode},{median:.3},{p95:.3},{maximum:.3},{},{},{},{},{},{}",
            stats.spaces,
            stats.choices,
            stats.material_options,
            stats.edges,
            route_count.len(),
            route_count,
        );
    }
}

fn parse_args() -> Result<Config, String> {
    let mut config = Config {
        warmups: DEFAULT_WARMUPS,
        samples: DEFAULT_SAMPLES,
        reverse: false,
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--warmups" => {
                config.warmups = parse_count("--warmups", arguments.next())?;
            }
            "--samples" => {
                config.samples = parse_count("--samples", arguments.next())?;
            }
            "--reverse" => config.reverse = true,
            "--help" | "-h" => {
                println!("android-core-runner [--warmups N] [--samples N] [--reverse]");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }
    if config.samples == 0 {
        return Err("--samples must be greater than zero".to_owned());
    }
    Ok(config)
}

fn parse_count(option: &str, value: Option<String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{option} requires a value"))?
        .parse()
        .map_err(|_| format!("{option} requires a non-negative integer"))
}

fn benchmark_cases(data: &GameData) -> Vec<(&'static str, &'static str, SearchRequest)> {
    let mut cases = Vec::new();
    for (name, target) in [
        ("Pixie", demon_ids::PIXIE),
        ("High Pixie", demon_ids::HIGH_PIXIE),
        ("Shiva", demon_ids::SHIVA),
        ("Alice", demon_ids::ALICE),
        ("Satan", demon_ids::SATAN),
    ] {
        assert!(!is_natural_skill(data, target, skill_ids::AGI));
        let mut first_playthrough = supported_initial_skills(data, target);
        first_playthrough.push(skill_ids::AGI);
        first_playthrough.sort_unstable();
        first_playthrough.dedup();
        cases.push((
            name,
            "new_game_plus",
            SearchRequest {
                target,
                required_skills: vec![skill_ids::AGI],
                max_fusion_depth: 4,
            },
        ));
        cases.push((
            name,
            "first_playthrough",
            SearchRequest {
                target,
                required_skills: first_playthrough,
                max_fusion_depth: 4,
            },
        ));
    }
    cases
}

fn is_natural_skill(data: &GameData, demon: DemonId, skill: SkillId) -> bool {
    data.demons()
        .get(demon)
        .unwrap()
        .natural_skills
        .iter()
        .any(|natural| natural.skill == skill)
}

fn supported_initial_skills(data: &GameData, demon: DemonId) -> Vec<SkillId> {
    data.demons()
        .get(demon)
        .unwrap()
        .natural_skills
        .iter()
        .filter(|natural| matches!(natural.acquisition, SkillAcquisition::Initial { .. }))
        .map(|natural| natural.skill)
        .filter(|skill| {
            data.skills().get(*skill).is_some_and(|skill| {
                skill.category != SkillCategory::Innate
                    && !skill.flags.magatsuhi
                    && !skill.flags.item_only
            })
        })
        .collect()
}

fn graph_stats(selector: &RouteSelector) -> GraphStats {
    let mut stats = GraphStats::default();
    let mut visited = HashSet::new();
    for space in &selector.routes {
        visit_space(space, &mut visited, &mut stats);
    }
    stats
}

fn visit_space(
    space: &Rc<RouteSpace>,
    visited: &mut HashSet<*const RouteSpace>,
    stats: &mut GraphStats,
) {
    if !visited.insert(Rc::as_ptr(space)) {
        return;
    }
    stats.spaces += 1;
    stats.choices += space.choices.len();
    for choice in &space.choices {
        if let RouteChoice::Fusion(fusion) = choice {
            stats.material_options += fusion.materials.len();
            for material in &fusion.materials {
                stats.edges += material.routes.len();
                for child in material.routes.iter() {
                    visit_space(child, visited, stats);
                }
            }
        }
    }
}
