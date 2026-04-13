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
  version "0.1.1"

  # ---------------------------------------------------------------------------
  # Prebuilt binaries — updated automatically by the release workflow.
  # ---------------------------------------------------------------------------

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-aarch64-apple-darwin.tar.gz"
      sha256 "a1574f54b68acd3f58d6d445dc103e290aa624cbfab7e7e4aecbbf150492109d"
    else
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-x86_64-apple-darwin.tar.gz"
      sha256 "0472bcbb58c48130a95e6f92291e07bc77db67d1082e4a66c191ef386e1fa567"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c3d366949b37314c091d670a8f6101b8f206c69d7bf91014d7d7072c79d21bf6"
    else
      url "https://github.com/aibunny/agent-ssh/releases/download/v#{version}/agent-ssh-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "b627d46d320e9f4a620e9409b1e77c76ee60ab9c2571d8343790c16be7727058"
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
