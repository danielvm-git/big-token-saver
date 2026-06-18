// Fixture: TypeScript greeter (bts-map snapshot test)

interface Greeter {
    greet(name: string): string;
}

class SimpleGreeter implements Greeter {
    greet(name: string): string {
        return `Hello, ${name}!`;
    }
}

function main(): void {
    const g = new SimpleGreeter();
    const msg = g.greet("world");
    console.log(msg);
}
