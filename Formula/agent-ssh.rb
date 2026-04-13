# Homebrew formula for agent-ssh.
#
# Users install via the tap in this repository:
#
#   brew tap aibunny/agent-ssh https://github.com/aibunny/agent-ssh
#   brew install agent-ssh
#
# To upgrade:
#   brew upgrade agent-ssh
#
# For development / unreleased builds, install from source:
#   brew install --build-from-source Formula/agent-ssh.rb

class AgentSsh < Formula
  desc "Security-first SSH broker — run named commands on named servers without exposing credentials"
  homepage "https://github.com/aibunny/agent-ssh"
  license "MIT"
  version "0.1.0"

  # ---------------------------------------------------------------------------
  # Prebuilt binaries — updated automatically by the release workflow.
  # ---------------------------------------------------------------------------

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_MACOS_ARM64"
    else
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_MACOS_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_LINUX_ARM64"
    else
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64"
    end
  end

  # ---------------------------------------------------------------------------
  # Source-based fallback (used by --build-from-source and Homebrew CI).
  # ---------------------------------------------------------------------------

  head do
    url "https://github.com/aibunny/agent-ssh.git", branch: "main"
    depends_on "rust" => :build
  end

  # ---------------------------------------------------------------------------

  def install
    if build.head?
      system "cargo", "install", *std_cargo_args(path: "crates/cli")
    else
      bin.install "agent-ssh"
    end
  end

  def caveats
    <<~EOS
      Quick start:
        agent-ssh init                     # create agent-ssh.toml
        agent-ssh config validate          # check config
        agent-ssh exec --server my-server --profile disk

      Full documentation:
        #{homepage}#readme
    EOS
  end

  test do
    # Smoke-test: init should write a valid TOML file.
    system bin/"agent-ssh", "init", "--output", testpath/"agent-ssh.toml"
    assert_predicate testpath/"agent-ssh.toml", :exist?

    output = shell_output("#{bin}/agent-ssh --version")
    assert_match version.to_s, output
  end
end
