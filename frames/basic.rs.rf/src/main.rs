/// Project $name$
/// 
/// Author: $param.author_name$ <$param.author_email_lowercase$>
///


fn main() {
    println!("Welcome to {} version {}", "$name$", "$version$");

    // <% if param.with_serde %>
    println!("aplikasi ini mendukung serde!");
    // <% endif %>
}
