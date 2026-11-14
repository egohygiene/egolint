function inspect(object, code) {
  return <p>{object.eval(code)}</p>;
}
