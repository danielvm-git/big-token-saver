// Fixture: Java greeter (bts-map snapshot test)
public class Greeter {
    public String greet(String name) {
        return "Hello, " + name + "!";
    }

    public static void main(String[] args) {
        Greeter g = new Greeter();
        System.out.println(g.greet("world"));
    }
}
