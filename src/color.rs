pub fn apply_colors(input: &str) -> String {
    input
        .replace("{RESET}", "\x1b[0m")
        .replace("{BOLD}", "\x1b[1m")
        .replace("{DIM}", "\x1b[2m")
        .replace("{UNDERLINE}", "\x1b[4m")
        .replace("{RED}", "\x1b[31m")
        .replace("{GREEN}", "\x1b[32m")
        .replace("{YELLOW}", "\x1b[33m")
        .replace("{BLUE}", "\x1b[34m")
        .replace("{MAGENTA}", "\x1b[35m")
        .replace("{CYAN}", "\x1b[36m")
        .replace("{WHITE}", "\x1b[37m")
}
