package salvo.codegen;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

import software.amazon.smithy.build.SmithyBuild;
import software.amazon.smithy.build.SmithyBuildResult;
import software.amazon.smithy.build.model.SmithyBuildConfig;

/**
 * Runs smithy-build over {@code smithy-build.json} (the working directory is
 * {@code modules/aws}) and copies each projection's {@code salvo-client-codegen}
 * output into {@code salvo/}, where the generated files are checked in.
 */
public final class Main {
    private Main() {}

    public static void main(String[] args) {
        Path config = Paths.get(args.length > 0 ? args[0] : "smithy-build.json").toAbsolutePath();
        Path root = config.getParent();
        SmithyBuildConfig buildConfig = SmithyBuildConfig.load(config);
        // Validate every imported model first, so a model that does not load
        // says why instead of producing an empty projection.
        for (var projection : buildConfig.getProjections().values()) {
            for (String imp : projection.getImports()) {
                var events = software.amazon.smithy.model.Model.assembler(Main.class.getClassLoader())
                        .discoverModels(Main.class.getClassLoader())
                        .addImport(root.resolve(imp))
                        .assemble()
                        .getValidationEvents(software.amazon.smithy.model.validation.Severity.ERROR);
                for (var event : events) {
                    System.err.println("error: " + imp + ": " + event);
                }
                if (!events.isEmpty()) {
                    System.exit(1);
                }
            }
        }
        SmithyBuild build = new SmithyBuild(buildConfig)
                .importBasePath(root)
                .outputDirectory(root.resolve("codegen/build/smithy"))
                .pluginClassLoader(Main.class.getClassLoader())
                // The trait definitions the AWS models use (rules engine, AWS
                // traits) are found on this classpath.
                .modelAssemblerSupplier(() -> software.amazon.smithy.model.Model.assembler(Main.class.getClassLoader())
                        .discoverModels(Main.class.getClassLoader()));
        SmithyBuildResult result = build.build();
        Path target = root.resolve("salvo");
        result.allArtifacts()
                .filter(p -> p.toString().contains("salvo-client-codegen"))
                .forEach(p -> copy(p, target));
        System.out.println("salvo-client-codegen: wrote " + target);
    }

    /** Copies one artifact to its place under {@code salvo/}. */
    private static void copy(Path artifact, Path target) {
        String s = artifact.toString().replace('\\', '/');
        int at = s.indexOf("/salvo-client-codegen/");
        String rel = s.substring(at + "/salvo-client-codegen/".length());
        Path out = target.resolve(rel);
        try {
            Files.createDirectories(out.getParent());
            Files.copy(artifact, out, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
            System.out.println("  " + target.getParent().relativize(out));
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }
}
