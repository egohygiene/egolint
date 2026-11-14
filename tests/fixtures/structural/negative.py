from collections.abc import Callable
import subprocess


def inspect(helper: Callable[..., str]) -> str:
    subprocess.run(["echo", "synthetic fixture"], shell=False)
    subprocess.run(helper(shell=True))
    return "subprocess.run(command, shell=True)"
