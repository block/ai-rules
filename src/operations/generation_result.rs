use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct GenerationResult {
    pub files_by_agent: BTreeMap<String, Vec<PathBuf>>,
}

impl GenerationResult {
    pub fn add_file(&mut self, agent: &str, file_path: PathBuf) {
        self.files_by_agent
            .entry(agent.to_string())
            .or_default()
            .push(file_path);
    }

    pub fn display(&self, current_dir: &Path) {
        if self.files_by_agent.is_empty() {
            return;
        }

        println!();
        for (agent, files) in &self.files_by_agent {
            if files.is_empty() {
                continue;
            }

            println!("    {agent}:");

            let mut files_by_dir: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
            for file in files {
                let relative_path = file.strip_prefix(current_dir).unwrap_or(file);
                let dir = relative_path.parent().unwrap_or(Path::new("."));
                files_by_dir
                    .entry(dir.to_path_buf())
                    .or_default()
                    .push(relative_path.to_path_buf());
            }

            let mut sorted_dirs: Vec<_> = files_by_dir.keys().collect();
            sorted_dirs.sort();

            for (i, dir) in sorted_dirs.iter().enumerate() {
                let files_in_dir = &files_by_dir[*dir];
                let is_last_dir = i == sorted_dirs.len() - 1;

                for (j, file) in files_in_dir.iter().enumerate() {
                    let is_last_file = j == files_in_dir.len() - 1;
                    let is_last_overall = is_last_dir && is_last_file;

                    let prefix = if is_last_overall {
                        "        └── "
                    } else {
                        "        ├── "
                    };

                    let full_path = current_dir.join(file);
                    if full_path.is_symlink() {
                        match fs::read_link(&full_path) {
                            Ok(target) => {
                                println!("{}{} -> {}", prefix, file.display(), target.display())
                            }
                            Err(_) => println!("{}{} (broken symlink)", prefix, file.display()),
                        }
                    } else {
                        println!("{}{}", prefix, file.display());
                    }
                }
            }

            println!();
        }
    }
}
