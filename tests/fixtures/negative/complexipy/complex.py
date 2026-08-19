def deeply_nested(values):
    total = 0
    for value in values:
        if value > 0:
            if value % 2 == 0:
                for candidate in range(value):
                    if candidate % 3 == 0:
                        if candidate > 10:
                            total += candidate
    return total
