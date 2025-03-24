if !variable::get("project-name").starts_with("sidecar-") {
    abort("Project name must start with 'sidecar-'")
}
