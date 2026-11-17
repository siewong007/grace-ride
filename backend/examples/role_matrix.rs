//! Throwaway helper: prints the default permission set of every built-in role.

fn main() {
    for role in grace_ride::perms::SYSTEM_ROLES {
        let mut perms: Vec<String> = grace_ride::perms::defaults(role)
            .into_iter()
            .map(|p| p.key())
            .collect();
        perms.sort();
        println!("{role}\t{}", perms.join(" "));
    }
}
