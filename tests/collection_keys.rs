use some_validation::CollectionValidationPipeline;
#[derive(PartialEq, Eq, Hash)]
struct Key(String);
struct Row {
    key: Key,
    label: String,
}
#[test]
fn borrowed_non_clone_keys_reject_duplicates_without_changing_rows() {
    let rows = [
        Row {
            key: Key("one".into()),
            label: "A".into(),
        },
        Row {
            key: Key("two".into()),
            label: "B".into(),
        },
    ];
    let pipeline = CollectionValidationPipeline::new().unique_by("key", |row: &Row| &row.key);
    assert!(pipeline.validate(&rows).is_ok());
    assert!(pipeline.validate(&[]).is_ok());
    assert_eq!(rows[0].label, "A");
    let duplicates = [
        Row {
            key: Key("one".into()),
            label: "A".into(),
        },
        Row {
            key: Key("one".into()),
            label: "B".into(),
        },
    ];
    assert!(pipeline.validate(&duplicates).is_err());
}
#[test]
fn borrowed_string_views_can_define_uniqueness_policy() {
    let pipeline =
        CollectionValidationPipeline::new().unique_by("name", |value: &String| value.trim());
    assert!(pipeline
        .validate(&["name".into(), " name ".into()])
        .is_err());
    assert!(pipeline.validate(&["name".into(), "other".into()]).is_ok());
}
