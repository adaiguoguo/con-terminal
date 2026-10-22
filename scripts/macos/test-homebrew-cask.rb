# frozen_string_literal: true

# Run with `brew ruby scripts/macos/test-homebrew-cask.rb`.
# Load the actual release template without tapping, downloading or installing.
require "yaml"
require "cask/cask_loader"
require "simulate_system"

workflow_path = File.expand_path("../../.github/workflows/release-macos.yml", __dir__)
workflow = YAML.load_file(workflow_path)
steps = workflow.fetch("jobs").values.flat_map { |job| job.fetch("steps", []) }
step = steps.find { |entry| entry["name"] == "Update cask formula" }
raise "Homebrew generation step not found" unless step

template = step.fetch("run")[/cat > "Casks\/\$\{cask\}\.rb" <<EOF\n(.*?)\nEOF/m, 1]
raise "Homebrew cask heredoc not found" unless template

[
  ["con-beta", "con Beta", "0.1.0-beta.142", ".beta"],
  ["con", "con", "0.1.0", ""],
].each do |token, app_name, version, bundle_suffix|
  values = {
    "cask" => token,
    "app_name" => app_name,
    "version" => version,
    "dmg_prefix" => app_name.tr(" ", "-"),
    "arm64_sha" => "a" * 64,
    "x86_64_sha" => "b" * 64,
    "bundle_suffix" => bundle_suffix,
  }
  content = template.gsub(/\$\{([a-z0-9_]+)\}/) { values.fetch(Regexp.last_match(1)) }
  raise "Unexpanded shell variable" if content.include?("${")
  raise "CLI artifact missing" unless content.include?('target: "con-cli"')

  { arm: ["arm64", "a"], intel: ["x86_64", "b"] }.each do |arch, (suffix, digest)|
    # This evaluates cask metadata only; it does not build or execute binaries.
    Homebrew::SimulateSystem.with(arch: arch) do
      cask = Cask::CaskLoader::FromContentLoader.new(content).load(config: nil)
      raise "Wrong cask token" unless cask.token == token
      raise "Wrong cask version" unless cask.version.to_s == version
      raise "Cask must remain macOS-only" unless cask.depends_on.requires_macos?
      expected_url = "https://github.com/nowledge-co/con-terminal/releases/download/" \
                     "v#{version}/#{values.fetch('dmg_prefix')}-#{version}-macos-#{suffix}.dmg"
      raise "Wrong architecture URL" unless cask.url.to_s == expected_url
      raise "Wrong architecture checksum" unless cask.sha256.to_s == digest * 64
      puts "#{token} (#{suffix}): generated cask loads with matching URL and checksum"
    end
  end
end
