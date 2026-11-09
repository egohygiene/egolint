import subprocess


def inspect():
    subprocess.run(["echo", "synthetic fixture"], shell=True)
