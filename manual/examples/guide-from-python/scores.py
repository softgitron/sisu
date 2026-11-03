# Each player's best and average points, from the lines of scores.txt.


class Player:
    def __init__(self, name):
        self.name = name
        self.points = []

    def best(self):
        return max(self.points) if self.points else None


def parse(line):
    name, points = line.split()
    return name, int(points)


def main():
    players = {}
    with open("scores.txt") as f:
        for line in f:
            try:
                name, points = parse(line)
            except ValueError:
                print(f"skipped: {line.strip()}")
                continue
            players.setdefault(name, Player(name)).points.append(points)
    for player in players.values():
        average = sum(player.points) / len(player.points)
        print(f"{player.name}: best {player.best()}, average {average:.1f}")


main()
