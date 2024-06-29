use std::fs;

use regex::Regex;

fn leftpad(text: &str, padding_count: usize) -> String {
    let re = Regex::new(r"(?m)^").unwrap();
    let padding = " ".repeat(padding_count);
    re.replace_all(text, format!("{}$0", padding).as_str())
        .to_string()
}

fn loadfile<P: AsRef<std::path::Path>>(loc: P, filename: &str) -> Option<String> {
    let file_to_check = loc.as_ref().join(filename).with_extension("yaml");
    let file = fs::File::open(&file_to_check);

    if let Ok(mut f) = file {
        let mut data = String::new();
        std::io::Read::read_to_string(&mut f, &mut data).unwrap();
        Some(data)
    } else {
        println!("template file not found {:?}", file_to_check);
        None
    }
}

pub fn preprocess_yaml<P: AsRef<std::path::Path>>(text: &str, loc: P) -> String {
    // find all lines starting with <<something>>, these will go through the
    // configs/prebuilt. it'll insert it, matching the indentation of the input.

    let pattern = r"(?m)^( *)\[\[([^\[\]]+)\]\]";

    let re = Regex::new(&pattern).unwrap();

    let mut new_text = text.to_owned();

    for cap in re.captures_iter(&text) {
        // println!("cap {:?}", cap);
        let spaces = cap[1].len();
        let filename = cap[2].to_string();

        // open up the file
        let contents = loadfile(loc.as_ref(), &filename);

        if let Some(c) = contents {
            let padded = leftpad(&c, spaces);
            new_text = new_text.replace(&format!("{}[[{}]]", &cap[1], filename), &padded);
        }
    }
    new_text
}
