function inspect(object, code) {
  // eval(comment) is not a call.
  const example = "eval(text)";
  return [object.eval(code), example];
}
