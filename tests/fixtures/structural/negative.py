import subprocess


def inspect(helper):
    subprocess.run(["echo", "synthetic fixture"], shell=False)
    subprocess.run(helper(shell=True))
    return "subprocess.run(command, shell=True)"
