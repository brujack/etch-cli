use criterion::{black_box, criterion_group, criterion_main, Criterion};
use etch_lib::actions::{FileAction, FileLink};
use etch_lib::manifests::Manifest;

const MANIFEST_YAML: &str = r#"
name: dotfiles
labels:
  - development
  - macos
actions:
  - action: command.run
    command: echo
    args:
      - hello
  - action: command.run
    command: echo
    args:
      - world
  - action: command.run
    command: echo
    args:
      - done
"#;

const MANIFEST_TOML: &str = r#"
name = "dotfiles"
labels = ["development", "macos"]
"#;

fn bench_manifest_yaml(c: &mut Criterion) {
    c.bench_function("manifest_yaml", |b| {
        b.iter(|| serde_yaml_ng::from_str::<Manifest>(black_box(MANIFEST_YAML)).unwrap())
    });
}

fn bench_manifest_toml(c: &mut Criterion) {
    c.bench_function("manifest_toml", |b| {
        b.iter(|| toml::from_str::<Manifest>(black_box(MANIFEST_TOML)).unwrap())
    });
}

fn bench_file_link_resolve(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let files_dir = dir.path().join("files");
    std::fs::create_dir_all(&files_dir).unwrap();
    std::fs::write(files_dir.join(".zshrc"), b"").unwrap();
    std::fs::write(files_dir.join(".gitconfig"), b"").unwrap();

    let manifest = Manifest {
        root_dir: Some(dir.path().to_path_buf()),
        ..Default::default()
    };
    let link = FileLink::default();

    let mut group = c.benchmark_group("file_link_resolve");
    group.bench_function("single_dotfile", |b| {
        b.iter(|| {
            link.resolve(black_box(&manifest), black_box(".zshrc"))
                .unwrap()
        })
    });
    group.bench_function("nested_path", |b| {
        std::fs::create_dir_all(files_dir.join("config/git")).unwrap();
        std::fs::write(files_dir.join("config/git/config"), b"").unwrap();
        b.iter(|| {
            link.resolve(black_box(&manifest), black_box("config/git/config"))
                .unwrap()
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_manifest_yaml,
    bench_manifest_toml,
    bench_file_link_resolve
);
criterion_main!(benches);
