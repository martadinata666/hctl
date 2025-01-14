use crate::{
    commands::progressbar_my_default_style,
    customio::lazy_read,
    resolver::{inbuilt_resolvers, valid_resolv_domain},
    rules::{
        iterator_map_whitespce, regex_extract_basic, regex_subdomain_all,
        regex_valid_domain_permissive, regex_whitespace,
    },
    savers::{self, file_write, io_writer_out, return_saver},
    structs::HCTL,
};
use indicatif::ProgressIterator;
use itertools::*;
use rayon::prelude::*;
use regex::Regex;
use std::{
    collections::BTreeSet,
    fs::{read_dir, remove_file, File},
    io::{self, *},
    sync::{Arc, Mutex},
    usize,
};
// use yaml_rust::*;

/// This function reads file into memory then enables parallel processing
pub fn process_parallel_list_to_file(
    list_path: String,
    out_path: String,
    save_rejected: bool,
    format: String,
    dns: bool,
) -> (usize, usize) {
    let pattern_basic = regex_extract_basic();
    let pattern_valid_domain = regex_valid_domain_permissive();
    let pattern_whitespace = regex_whitespace();

    let file_opened = file_to_lines(list_path).unwrap();
    let reader = BufReader::new(file_opened);

    let mut writer_out = io_writer_out(out_path);

    let file_rejected = file_write("./rejected.txt".to_string()).unwrap();
    let mut writer_rejected = BufWriter::new(file_rejected);

    let arc_mux_set_rejected = Arc::new(Mutex::new(BTreeSet::new()));
    let mut count_entries: usize = 0;

    let saver_func = return_saver(format.clone());
    let saver_rejected_func = return_saver("linewise".to_string());

    match format.as_str() {
        "empty" | "loopback" => _ = writer_out.write_all(savers::HOSTLIST_SCHEME.as_bytes()),
        "unbound" => _ = writer_out.write_all(savers::UNBOUND_PRE.as_bytes()),
        _ => _ = writer_out.write_all(b"\n"),
    }

    // Closures are workaround for cannot & to mut value
    let invalid_domain = |word: &String| {
        let is_domain = pattern_valid_domain.is_match(word);
        if !is_domain {
            arc_mux_set_rejected.lock().unwrap().insert(word.clone());
        }
        return is_domain;
    };

    let mut save_out_entry = |word| {
        count_entries += 1;
        _ = writer_out.write_all(saver_func(word).as_bytes());
    };

    let mut save_rejected_all = || {
        arc_mux_set_rejected
            .lock()
            .unwrap()
            .iter()
            .for_each(|word| {
                _ = writer_rejected.write_all(saver_rejected_func(word).as_bytes());
            });
        _ = writer_rejected.flush();
    };

    let validate_dns = |word: &String| {
        if dns {
            let (isok, resolvernum) = valid_resolv_domain(word, inbuilt_resolvers());
            if !isok {
                let mut rejec = word.clone();
                rejec.push_str("\t# Domain reslution failed at resolver nr. ");
                rejec.push_str(resolvernum.to_string().as_str());
                arc_mux_set_rejected.lock().unwrap().insert(rejec);
            }
            return isok;
        }
        return true;
    };

    reader
        .lines()
        .map(|res| res.unwrap())
        .filter(|line| !line.starts_with('#'))
        .filter(|line| !line.eq(""))
        .collect::<BTreeSet<_>>()
        .par_iter()
        .map(|word| pattern_basic.replace_all(word, "").to_string())
        .map(|word| iterator_map_whitespce(&pattern_whitespace, word))
        .filter(|word| invalid_domain(word))
        .filter(|x| validate_dns(x))
        .collect::<BTreeSet<_>>()
        .iter()
        .progress_with_style(progressbar_my_default_style())
        .for_each(|word| save_out_entry(word));

    _ = writer_out.flush();

    if save_rejected {
        save_rejected_all();
    } else {
        drop(writer_rejected);
        _ = remove_file("./rejected.txt");
    }

    return (count_entries, arc_mux_set_rejected.lock().unwrap().len());
}

pub fn process_single_list_seq_file(
    list_path: String,
    out_path: String,
    save_rejected: bool,
    format: String,
    // dns: bool,
) -> (usize, usize) {
    // Declaration
    let pattern_basic = regex_extract_basic();
    let pattern_whitespace = regex_whitespace();
    let pattern_valid_domain = regex_valid_domain_permissive();

    let file_opened = file_to_lines(list_path).unwrap();
    let reader = BufReader::new(file_opened);

    let mut writer_out = io_writer_out(out_path);

    let file_rejected = file_write("./rejected.txt".to_string()).unwrap();
    let mut writer_rejected = BufWriter::new(file_rejected);

    let mut set_rejected: BTreeSet<String> = BTreeSet::new();
    let mut count_entries: usize = 0;

    let saver_func = return_saver(format.clone());
    let saver_rejected_func = return_saver("linewise".to_string());

    match format.as_str() {
        "empty" | "loopback" => _ = writer_out.write_all(savers::HOSTLIST_SCHEME.as_bytes()),
        "unbound" => _ = writer_out.write_all(savers::UNBOUND_PRE.as_bytes()),
        _ => _ = writer_out.write_all(b"\n"),
    }

    // Closures are workaround for cannot reference to mut value
    let mut invalid_domain = |word: &String| {
        let res = pattern_valid_domain.is_match(word);
        if !res {
            set_rejected.insert(word.clone());
        }
        return res;
    };

    let mut save_out_entry = |word| {
        count_entries += 1;
        _ = writer_out.write_all(saver_func(&word).as_bytes());
    };

    // let mut validate_dns = |word: &String| {
    //     if dns {
    //         let (isok, resolvernum) = valid_resolv_domain(word, inbuilt_resolvers());
    //         if !isok {
    //             let mut rejec = word.clone();
    //             rejec.push_str("\t# Domain reslution failed at resolver nr. ");
    //             rejec.push_str(resolvernum.to_string().as_str());
    //             set_rejected.insert(rejec);
    //         }
    //         return isok;
    //     }
    //     return true;
    // };

    // Processing
    reader
        .lines()
        .map(|result| result.unwrap())
        .filter(|line| !line.starts_with('#'))
        .map(|word| pattern_basic.replace_all(word.as_str(), "").to_string())
        .map(|word| iterator_map_whitespce(&pattern_whitespace, word))
        .unique()
        .filter(|word| invalid_domain(word))
        // .filter(|x| validate_dns(x))
        .sorted()
        .progress_with_style(progressbar_my_default_style())
        .for_each(|word| save_out_entry(word));

    _ = writer_out.flush();

    if save_rejected {
        set_rejected.iter().for_each(|word| {
            _ = writer_rejected.write_all(saver_rejected_func(word).as_bytes());
        });
        _ = writer_rejected.flush();
    } else {
        drop(writer_rejected);
        _ = remove_file("./rejected.txt");
    }

    return (count_entries, set_rejected.len());
}

pub fn process_single_list_to_set(list_path: &String) -> (BTreeSet<String>, BTreeSet<String>) {
    let pattern_basic = regex_extract_basic();
    let pattern_valid_domain = regex_valid_domain_permissive();
    let pattern_whitespace = regex_whitespace();

    let file_opened = file_to_lines(list_path.clone()).unwrap();
    let reader = BufReader::new(file_opened);

    let mut set_rejected: BTreeSet<String> = BTreeSet::new();

    //CLOSUERS
    let mut invalid_domain = |word: &String| {
        let res = pattern_valid_domain.is_match(word);
        if !res {
            let mut x: String = word.clone();
            if !pattern_whitespace.is_match(x.as_str()) {
                x.push_str("\t# source: ");
                x.push_str(list_path);
                set_rejected.insert(x);
            }
        }
        return res;
    };

    let set_cleaned = reader
        .lines()
        .map(|result| result.unwrap())
        .filter(|line| !line.starts_with('#'))
        .map(|word| pattern_basic.replace_all(word.as_str(), "").to_string())
        .map(|word| iterator_map_whitespce(&pattern_whitespace, word))
        .filter(|word| invalid_domain(word))
        .collect::<BTreeSet<_>>();

    return (set_cleaned, set_rejected);
}

pub fn process_multiple_lists_to_file(
    list_dir: String,
    out_path: String,
    save_rejected: bool,
    format: String,
    dns: bool,
) -> (usize, usize) {
    let mut writer_out = io_writer_out(out_path);
    let file_rejected = file_write("./rejected.txt".to_string()).unwrap();
    let mut writer_rejected = BufWriter::new(file_rejected);

    let arc_mux_set_rejected = Arc::new(Mutex::new(BTreeSet::new()));
    let mut count_entries: usize = 0;

    let saver_func = return_saver(format.clone());
    let saver_rejected_func = return_saver("linewise".to_string());

    match format.as_str() {
        "empty" | "loopback" => _ = writer_out.write_all(savers::HOSTLIST_SCHEME.as_bytes()),
        "unbound" => _ = writer_out.write_all(savers::UNBOUND_PRE.as_bytes()),
        _ => _ = writer_out.write_all(b"\n"),
    }

    // CLOSURES
    let extend_rejected_from_result = |set_cleared, set_rejected| {
        arc_mux_set_rejected.lock().unwrap().extend(set_rejected);
        return set_cleared;
    };

    let mut flush_rejected = || {
        arc_mux_set_rejected
            .lock()
            .unwrap()
            .iter()
            .for_each(|word| {
                _ = writer_rejected.write_all(saver_rejected_func(word).as_bytes());
            });
        _ = writer_rejected.flush();
    };

    let validate_dns = |word: &String| {
        if dns {
            let (isok, resolvernum) = valid_resolv_domain(word, inbuilt_resolvers());
            if !isok {
                let mut rejec = word.clone();
                rejec.push_str("\t# Domain reslution failed at resolver nr. ");
                rejec.push_str(resolvernum.to_string().as_str());
                arc_mux_set_rejected.lock().unwrap().insert(rejec);
            }
            return isok;
        }
        return true;
    };

    read_dir(list_dir.as_str())
        .unwrap()
        .filter_map(|result| result.ok())
        .map(|dir| dir.path().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .par_iter()
        .map(|line| process_single_list_to_set(line))
        .map(|(set_cleared, set_rejected)| extend_rejected_from_result(set_cleared, set_rejected))
        // .collect::<Vec<_>>()
        // .par_iter()
        .flatten()
        .filter(|x| validate_dns(x))
        .collect::<BTreeSet<_>>()
        .iter()
        .progress_with_style(progressbar_my_default_style())
        .for_each(|word| {
            count_entries += 1;
            _ = writer_out.write_all(saver_func(word).as_bytes());
        });
    _ = writer_out.flush();

    if save_rejected {
        flush_rejected();
    } else {
        drop(writer_rejected);
        _ = remove_file("./rejected.txt");
    }

    return (count_entries, arc_mux_set_rejected.lock().unwrap().len());
}

pub fn file_to_lines(path: String) -> io::Result<File> {
    let file = File::open(path)?;
    return Ok(file);
}

pub fn config_process_lists(
    path: String,
    out_path: String,
    use_intro: bool,
    save_rejected: bool,
    format: String,
    dns: bool,
) -> (usize, usize) {
    // let settings_as_str = read_to_string(file_to_lines(path).unwrap()).unwrap();
    let hctl_yaml: HCTL = serde_yaml::from_reader(file_to_lines(path).unwrap()).unwrap();
    // let parsed_settings_yaml_first = &parsed_settings_yaml[0];

    let mut writer_out = io_writer_out(out_path);

    let file_rejected = file_write("./rejected.txt".to_string()).unwrap();
    let mut writer_rejected = BufWriter::new(file_rejected);

    let arc_mux_set_rejected = Arc::new(Mutex::new(BTreeSet::new()));
    // let mut arc_mux_num = Arc::new(HoldNum::new(0));
    // let arc_mux_vec_resolvers = Arc::new(Mutex::new(many_resolvers_tls()));

    let mut count_entries: usize = 0;

    let saver_func = return_saver(format.clone());
    let saver_rejected_func = return_saver("linewise".to_string());

    let mut set_whitelist: BTreeSet<String> =
        hctl_yaml.whitelist.into_par_iter().collect::<BTreeSet<_>>();

    set_whitelist.extend(
        set_whitelist
            .clone()
            .into_par_iter()
            .map(|s| lazy_read(s.as_str()))
            .filter_map(|result| result.ok())
            .map(|(set_cleaned, _)| {
                return set_cleaned;
            })
            .collect::<Vec<_>>()
            .into_par_iter()
            .flatten()
            .collect::<BTreeSet<_>>(),
    );

    let subdomains_regex: Vec<Regex> = match hctl_yaml.settings.whitelist_include_subdomains {
        true => set_whitelist
            .iter()
            .map(|x| regex_subdomain_all(x))
            .collect(),
        false => Vec::new(),
    };
    // DEBUG
    //     hctl_yaml.whitelist.clone()
    //         .whitelist
    // .       .iter()
    //         .for_each(|x| println!("{}", x.as_str()));

    // CLOSURES
    let mut flush_rejected = || {
        arc_mux_set_rejected
            .lock()
            .unwrap()
            .iter()
            .for_each(|word| {
                _ = writer_rejected.write_all(saver_rejected_func(&word).as_bytes());
            });
        _ = writer_rejected.flush();
    };

    let extend_rejected_from_result = |set_cleared, set_rejected| {
        arc_mux_set_rejected.lock().unwrap().extend(set_rejected);
        return set_cleared;
    };

    let subdomains = |domain| {
        if subdomains_regex.len() > 0 {
            return !subdomains_regex
                .iter()
                .map(|x| x.is_match(domain))
                .find_or_first(|x| x == &true)
                .unwrap();
        }
        return true;
        // .unique()
        // .find_map(|x| x.is_match(domain));
    };

    let validate_dns = |word: &String| {
        if dns {
            let (isok, resolvernum) = valid_resolv_domain(word, inbuilt_resolvers());
            if !isok {
                let mut rejec = word.clone();
                rejec.push_str("\t# Domain reslution failed at resolver nr. ");
                rejec.push_str(resolvernum.to_string().as_str());
                arc_mux_set_rejected.lock().unwrap().insert(rejec);
            }
            return isok;
        }
        return true;
    };

    // let resolv_valid_reject = |domain| -> bool {
    //     // let p = arc_mux_vec_resolvers.lock().unwrap().push();
    //     // p.push(p.remove(0));
    //     // let x = arc_mux_num.lock().unwrap();
    //     // x = x + 1;
    //     let res = valid_resolv_domain(domain, many_resolvers_tls());
    //     let mut x = domain.clone();
    //     x.push_str("\t# Domain reslution failed at resolver nr. ");
    //     x.push_str(res.1.to_string().as_str());
    //     arc_mux_set_rejected.lock().unwrap().insert(x);
    //     println!("{}: {}", res.0, domain);
    //     return res.0;
    // };
    // Processing

    if use_intro {
        let sources_cloned: Vec<String> = hctl_yaml.remote_sources.clone().into_iter().collect();
        _ = writer_out.write_all("# This hostlist was assembled from other lists:\n".as_bytes());

        sources_cloned.iter().for_each(|line| {
            _ = writer_out.write_all("# \t- ".as_bytes());
            _ = writer_out.write_all(line.as_bytes());
            _ = writer_out.write_all("\n".as_bytes());
        });
    }

    match format.as_str() {
        "empty" | "loopback" => _ = writer_out.write_all(savers::HOSTLIST_SCHEME.as_bytes()),
        "unbound" => _ = writer_out.write_all(savers::UNBOUND_PRE.as_bytes()),
        _ => _ = writer_out.write_all(b"\n"),
    }

    hctl_yaml
        .remote_sources
        .into_par_iter()
        .map(|s| lazy_read(s.as_str()))
        .filter_map(|result| result.ok())
        .map(|(set_cleaned, set_rejected)| extend_rejected_from_result(set_cleaned, set_rejected))
        // .collect::<Vec<_>>()
        // .into_par_iter()
        .flatten()
        .collect::<BTreeSet<_>>()
        .par_iter()
        .filter(|x| subdomains(x))
        .filter(|x| validate_dns(x))
        .collect::<BTreeSet<_>>()
        .iter()
        .progress_with_style(progressbar_my_default_style())
        .for_each(|word| {
            count_entries += 1;
            _ = writer_out.write_all(saver_func(word).as_bytes());
        });

    _ = writer_out.flush();

    if save_rejected {
        flush_rejected();
    } else {
        drop(writer_rejected);
        _ = remove_file("./rejected.txt");
    }
    return (count_entries, arc_mux_set_rejected.lock().unwrap().len());
}
