import subprocess


def inspect() -> None:
    subprocess.run(["echo", "synthetic fixture"], shell=True)
