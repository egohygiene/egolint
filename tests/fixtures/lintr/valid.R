#' Format a greeting for a supplied name.
#'
#' @param name Character value containing the name to greet.
#'
#' @return A formatted greeting.
format_greeting <- function(name) {
  normalized_name <- trimws(name)

  if (!nzchar(normalized_name)) {
    return("Hello.")
  }

  sprintf("Hello, %s.", normalized_name)
}
