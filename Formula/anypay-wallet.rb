class AnypayWallet < Formula
  desc "Anypay wallet client"
  homepage "https://github.com/anypay/anypay"
  version "0.1.0" # Update this with your version

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/anypay/anypay/releases/download/v#{version}/anypay-macOS-aarch64.tar.gz"
      sha256 "UPDATE_WITH_ACTUAL_SHA" # Update after release
    else
      url "https://github.com/anypay/anypay/releases/download/v#{version}/anypay-macOS-x86_64.tar.gz"
      sha256 "UPDATE_WITH_ACTUAL_SHA" # Update after release
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/anypay/anypay/releases/download/v#{version}/anypay-Linux-aarch64.tar.gz"
      sha256 "UPDATE_WITH_ACTUAL_SHA" # Update after release
    else
      url "https://github.com/anypay/anypay/releases/download/v#{version}/anypay-Linux-x86_64.tar.gz"
      sha256 "UPDATE_WITH_ACTUAL_SHA" # Update after release
    end
  end

  def install
    bin.install "anypay-wallet"
  end

  test do
    system "#{bin}/anypay-wallet", "--version"
  end
end 