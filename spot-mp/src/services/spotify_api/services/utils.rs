pub fn parse_release_year(release_date: Option<&str>) -> Option<i32> {
    release_date
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse::<i32>().ok())
}