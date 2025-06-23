import sys

def parse_la_format(text):
    lines = [line for line in text.strip().splitlines() if line.strip() and not line.strip().startswith('#')]
    header = lines[0]
    n_jobs, n_machines = map(int, header.strip().split())
    job_lines = lines[1:]

    machines = []
    durations = []

    for line in job_lines:
        tokens = list(map(int, line.strip().split()))
        job_machines = []
        job_durations = []

        for i in range(0, len(tokens), 2):
            machine = tokens[i] + 1  # convert to 1-based index
            duration = tokens[i + 1]
            job_machines.append(machine)
            job_durations.append(duration)

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

# === Example Usage ===
if __name__ == "__main__":
    f = open(sys.argv[1])
    raw_data = f.read()
    n_jobs, n_machines, machines, durations = parse_la_format(raw_data)
    write_dzn(sys.argv[2], n_jobs, n_machines, machines, durations)
    print(f"✅ {sys.argv[2]}.dzn generated")

