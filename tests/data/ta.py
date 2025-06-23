import sys

def parse_taillard_text(data: str):
    lines = data.strip().splitlines()
    n_jobs, n_machines = map(int, lines[0].split())

    machines = []
    durations = []

    for line in lines[1:]:
        tokens = list(map(int, line.strip().split()))
        job_machines = [tokens[i] + 1 for i in range(0, len(tokens), 2)]  # 1-based
        job_durations = [tokens[i] for i in range(1, len(tokens), 2)]

        if len(job_machines) != n_machines or len(job_durations) != n_machines:
            raise ValueError("Mismatch in number of machines or durations")

        machines.append(job_machines)
        durations.append(job_durations)

    return n_jobs, n_machines, machines, durations


def write_dzn(filename, n_jobs, n_machines, machines, durations):
    with open(filename + ".dzn", "w") as f:
        f.write(f"n_jobs = {n_jobs};\n")
        f.write(f"n_machines = {n_machines};\n")

        f.write("machines = [")
        for row in machines:
            f.write("| " + (", ".join(str(x) for x in row)) + "\n")
        f.write("|];\n")

        f.write("durations = [")
        for row in durations:
            f.write("| " + (", ".join(str(x) for x in row)) + "\n")
        f.write("|];\n")


if __name__ == "__main__":
    f = open(sys.argv[1])
    raw_data = f.read()
    n_jobs, n_machines, machines, durations = parse_taillard_text(raw_data)
    write_dzn(sys.argv[2], n_jobs, n_machines, machines, durations)
    print(f"✅ Wrote: {sys.argv[2]}.dzn")

    f.close()
