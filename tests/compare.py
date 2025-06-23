import os

files = None
types = []

# gets the results required from an array of lines (from the file)
#
# you can insert more variables and if statements and return them if needed
def get_results(lines):
    t_end = None
    conflicts = None
    lbd = None
    time = None
    proven_optimality = False
    
    for line in lines:
        if "====" in line:
            proven_optimality = True
        
        line = line.split("=")

        if "t_end" in line[0]:
            t_end = line[1][1:]
        elif "%%%mzn-stat: engineStatisticsNumConflicts" in line[0]:
            conflicts = line[1]
        elif "%%%mzn-stat: learnedClauseStatisticsAverageLbd" in line[0]:
            lbd = line[1]
        elif "time elapsed" in line[0]:
            time = line[0].split(": ")[1]
            
    return t_end, conflicts, lbd, time, proven_optimality

for folder in sorted(os.listdir("./output")):
    types.append(folder)

    # this is to create the files that will be uploaded
    #
    # in this case i take the intersection of files available, so i have full solutions for everything
    #
    # feel free to change it :D
    if files is None:
        files = set(os.listdir("./output/" + folder))
    else:
        files = files.intersection(os.listdir("./output/" + folder))

if files == None:
    print("Nothing has been run yet")
    exit(1)

for f in sorted(files):
    print(f"------ Dataset: {f} ------")

    for type in types:
        solution = open(f"./output/{type}/{f}")

        res = get_results(solution.read().split("\n"))
        print(f"{type}: {res}")
                    
        solution.close()

    print()
