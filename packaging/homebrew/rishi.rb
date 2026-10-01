class Rishi < Formula
  desc "The Universal Programming Language for AI, Systems, and Cloud"
  homepage "https://rishikesh.lang"
  url "https://github.com/kingroyale537/rishikesh/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/rishi_cli")
  end

  test do
    (testpath/"test.rk").write <<~EOS
      fn main():
          println("Hello from Rishikesh via Homebrew!")
      main()
    EOS
    assert_match "Hello from Rishikesh via Homebrew!", shell_output("#{bin}/rishi run test.rk")
  end
end
