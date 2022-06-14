# To update nix-prefetch-git https://github.com/NixOS/nixpkgs
import ((import <nixpkgs> {}).fetchFromGitHub {
  owner = "NixOS";
  repo = "nixpkgs";
  rev = "7ec99ea7cf9616ef4c6e835710202623fcb846e7";
  sha256  = "1cp4sb4v1qzb268h2ky7039lf1gwkrs757q6gv2wd2hs65kvf1q7";
})
