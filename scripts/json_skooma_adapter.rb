#!/usr/bin/env ruby
# frozen_string_literal: true

# Copyright 2026 Ego Hygiene
# SPDX-License-Identifier: MIT

require 'digest'
require 'json'
require 'optparse'
require 'pathname'

ADAPTER_VERSION = '1.0.0'
EXPECTED_JSON_SKOOMA_VERSION = '0.2.7'
EXPECTED_GEM_VERSIONS = {
  'json_skooma' => EXPECTED_JSON_SKOOMA_VERSION,
  'bigdecimal' => '4.1.3',
  'hana' => '1.3.7',
  'regexp_parser' => '2.12.0',
  'uri-idna' => '0.3.1',
  'zeitwerk' => '2.8.3',
}.freeze
MAX_INPUT_BYTES = 1_048_576
SUPPORTED_DIALECTS = {
  'https://json-schema.org/draft/2019-09/schema' => '2019-09',
  'https://json-schema.org/draft/2020-12/schema' => '2020-12',
}.freeze
REFERENCE_KEYWORDS = %w[$ref $dynamicRef $recursiveRef].freeze

class ConfigurationError < StandardError; end

def parse_options
  options = {
    workspace: Pathname.pwd,
    report: '.reports/egolint/complementary/json-skooma/latest.json',
  }
  OptionParser.new do |parser|
    parser.banner = 'Usage: json_skooma_adapter.rb [options]'
    parser.on('--workspace PATH') { |value| options[:workspace] = Pathname.new(value) }
    parser.on('--config PATH') { |value| options[:config] = Pathname.new(value) }
    parser.on('--report PATH') { |value| options[:report] = value }
    parser.on('--version') { options[:version] = true }
  end.parse!
  options
end

def within?(root, candidate)
  candidate.to_s == root.to_s || candidate.to_s.start_with?("#{root}#{File::SEPARATOR}")
end

def repository_file(workspace, raw_path)
  relative = Pathname.new(raw_path)
  raise ConfigurationError, 'input path must be repository-relative' if relative.absolute?
  raise ConfigurationError, 'input path cannot contain parent traversal' if relative.each_filename.include?('..')

  resolved = (workspace + relative).realpath
  raise ConfigurationError, 'input path escapes the workspace' unless within?(workspace, resolved)
  raise ConfigurationError, 'input path must be a regular file' unless resolved.file?
  raise ConfigurationError, "input exceeds the #{MAX_INPUT_BYTES}-byte limit" if resolved.size > MAX_INPUT_BYTES

  resolved
rescue Errno::ENOENT, Errno::EACCES => error
  raise ConfigurationError, 'input path is unavailable', cause: error
end

def load_json(path)
  raise ConfigurationError, "input exceeds the #{MAX_INPUT_BYTES}-byte limit" if path.size > MAX_INPUT_BYTES

  JSON.parse(path.read(encoding: 'UTF-8'))
rescue JSON::ParserError, EncodingError => error
  raise ConfigurationError, 'input must be valid UTF-8 JSON', cause: error
end

def each_reference(value, &block)
  case value
  when Hash
    REFERENCE_KEYWORDS.each do |keyword|
      reference = value[keyword]
      yield reference if reference.is_a?(String)
    end
    value.each_value { |child| each_reference(child, &block) }
  when Array
    value.each { |child| each_reference(child, &block) }
  end
end

def validate_references(schema)
  each_reference(schema) do |reference|
    next if reference.start_with?('#')

    raise ConfigurationError, 'only fragment-local schema references are supported'
  end
end

def dialect_for(schema)
  raw = schema['$schema']
  normalized = raw.is_a?(String) ? raw.delete_suffix('#') : nil
  dialect = SUPPORTED_DIALECTS[normalized]
  raise ConfigurationError, 'schema must declare draft 2019-09 or 2020-12' unless dialect

  dialect
end

def keyword_from(path)
  segment = path.to_s.split('/').last.to_s
  segment.gsub('~1', '/').gsub('~0', '~')
end

def normalized_findings(output)
  Array(output['errors']).map do |error|
    schema_path = error.fetch('keywordLocation', '')
    keyword = keyword_from(schema_path)
    {
      'rule_id' => "json-skooma:#{keyword.empty? ? 'schema' : keyword}",
      'keyword' => keyword,
      'instance_path' => error.fetch('instanceLocation', ''),
      'schema_path' => schema_path,
    }
  end
end

def write_report(destination, report)
  rendered = JSON.pretty_generate(report) + "\n"
  if destination == '-'
    $stdout.write(rendered)
    return
  end

  path = Pathname.new(destination)
  path.dirname.mkpath
  path.write(rendered, encoding: 'UTF-8')
end

def error_report(status, reason, started_at)
  {
    'schema_version' => 1,
    'tool_id' => 'EGOLINT_JSON_SKOOMA',
    'adapter_version' => ADAPTER_VERSION,
    'validator' => { 'name' => 'json_skooma', 'version' => EXPECTED_JSON_SKOOMA_VERSION },
    'status' => status,
    'reason' => reason,
    'duration_seconds' => (Process.clock_gettime(Process::CLOCK_MONOTONIC) - started_at).round(6),
    'network' => 'disabled-by-contract',
    'mappings' => [],
  }
end

def load_validator
  EXPECTED_GEM_VERSIONS.each { |name, version| gem name, "=#{version}" }
  require 'json_skooma'
  actual = EXPECTED_GEM_VERSIONS.keys.to_h do |name|
    [name, Gem.loaded_specs.fetch(name).version.to_s]
  end
  message = 'activated JSONSkooma gem graph does not match the adapter pins'
  raise ConfigurationError, message unless actual == EXPECTED_GEM_VERSIONS

  JSONSkooma.create_registry('2019-09', '2020-12', assert_formats: true)
end

def load_schema(workspace, schema_name)
  schema_path = repository_file(workspace, schema_name)
  schema_value = load_json(schema_path)
  raise ConfigurationError, 'schema input must contain one object' unless schema_value.is_a?(Hash)

  validate_references(schema_value)
  dialect = dialect_for(schema_value)
  stable_uri = "urn:egolint:json-skooma:#{Digest::SHA256.file(schema_path).hexdigest}"
  schema = JSONSkooma::JSONSchema.new(schema_value, uri: stable_uri)
  message = 'schema does not validate against its declared metaschema'
  raise ConfigurationError, message unless schema.validate.valid?

  [schema, dialect]
end

def evaluate_instance(workspace, schema, instance_name)
  instance_path = repository_file(workspace, instance_name)
  result = schema.evaluate(load_json(instance_path))
  {
    'instance' => instance_name,
    'valid' => result.valid?,
    'findings' => normalized_findings(result.output(:basic)),
  }
end

def evaluate_mapping(workspace, mapping)
  schema_name = mapping.fetch('schema')
  schema, dialect = load_schema(workspace, schema_name)
  instance_reports = mapping.fetch('instances').map do |instance_name|
    evaluate_instance(workspace, schema, instance_name)
  end
  {
    'schema' => schema_name,
    'dialect' => dialect,
    'instances' => instance_reports,
  }
end

def completed_report(mapping_reports, config_path, started_at)
  failed = mapping_reports.any? do |mapping|
    mapping['instances'].any? { |instance| !instance['valid'] }
  end
  {
    'schema_version' => 1,
    'tool_id' => 'EGOLINT_JSON_SKOOMA',
    'adapter_version' => ADAPTER_VERSION,
    'validator' => { 'name' => 'json_skooma', 'version' => JSONSkooma::VERSION },
    'status' => failed ? 'failed_findings' : 'passed',
    'reason' => failed ? 'schema-validation-findings' : 'all-instances-valid',
    'configuration_digest' => Digest::SHA256.file(config_path).hexdigest,
    'represented_commit' => ENV.fetch('EGOLINT_REPRESENTED_COMMIT', 'unknown'),
    'duration_seconds' => (Process.clock_gettime(Process::CLOCK_MONOTONIC) - started_at).round(6),
    'network' => ENV.fetch('EGOLINT_JSON_SKOOMA_NETWORK', 'none'),
    'mappings' => mapping_reports,
  }
end

def run(options, started_at)
  raise ConfigurationError, 'a config path is required' unless options[:config]

  workspace = options[:workspace].realpath
  config_path = options[:config].realpath
  raise ConfigurationError, 'config path escapes the workspace' unless within?(workspace, config_path)

  config = load_json(config_path)
  mappings = config.fetch('mappings')
  load_validator
  mapping_reports = mappings.map { |mapping| evaluate_mapping(workspace, mapping) }
  completed_report(mapping_reports, config_path, started_at)
end

options = parse_options
if options[:version]
  puts "egolint-json-skooma #{ADAPTER_VERSION} (json_skooma #{EXPECTED_JSON_SKOOMA_VERSION})"
  exit 0
end

started_at = Process.clock_gettime(Process::CLOCK_MONOTONIC)
begin
  report = run(options, started_at)
  write_report(options[:report], report)
  exit(report['status'] == 'passed' ? 0 : 1)
rescue ConfigurationError, KeyError, TypeError => error
  write_report(options[:report], error_report('invalid_configuration', error.message, started_at))
  exit 2
rescue StandardError => error
  write_report(options[:report], error_report('execution_error', error.class.name, started_at))
  exit 3
end
