function inspect(object: { eval: (value: string) => unknown }, code: string) {
  const example: string = "eval(text)";
  return [object.eval(code), example];
}
