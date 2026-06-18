# Fixture: Python greeter (bts-map snapshot test)

class Greeter:
    def greet(self, name):
        return f"Hello, {name}!"

def main():
    g = Greeter()
    result = g.greet("world")
    print(result)
