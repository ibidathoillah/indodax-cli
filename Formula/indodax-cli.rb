class IndodaxCli < Formula
  desc "Command-line interface for the Indodax cryptocurrency exchange"
  homepage "https://github.com/ibidathoillah/indodax-cli"
  url "https://github.com/ibidathoillah/indodax-cli/archive/refs/tags/v0.1.45.tar.gz"
  sha256 "b82879a41648044a27308fea94dd0c0e9065e9db101f3025902b929338c4196a"
  license "MIT"
  head "https://github.com/ibidathoillah/indodax-cli.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".", features: "cli,mcp,server")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/indodax-cli --version")
  end
end
