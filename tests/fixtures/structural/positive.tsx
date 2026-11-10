function inspect(code: string) {
  return <p>{eval(code)}</p>;
}
