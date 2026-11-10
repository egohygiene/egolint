function inspect(object: { eval: (value: string) => unknown }, code: string) {
  return <p>{object.eval(code)}</p>;
}
