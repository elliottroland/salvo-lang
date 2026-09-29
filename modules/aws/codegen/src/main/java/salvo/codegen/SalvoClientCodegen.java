package salvo.codegen;

import java.util.Map;

import software.amazon.smithy.build.PluginContext;
import software.amazon.smithy.build.SmithyBuildPlugin;

/**
 * The {@code salvo-client-codegen} smithy-build plugin: one AWS service's
 * Smithy model in, a Salvo module and the host glue that wraps the platform's
 * AWS SDK out (modules/aws/DESIGN.md, decisions D1–D8 and 14–18).
 *
 * <p>Settings, from the plugin's entry in {@code smithy-build.json}:
 * <ul>
 *   <li>{@code service} — the service shape id;</li>
 *   <li>{@code module} — the Salvo module path, e.g. {@code aws.sqs};</li>
 *   <li>{@code effect} — the effect's name, e.g. {@code Sqs};</li>
 *   <li>{@code operations} — the operations to generate, by shape name;</li>
 *   <li>{@code rustCrate} — the aws-sdk-rust crate, e.g. {@code aws_sdk_sqs};</li>
 *   <li>{@code kotlinPackage} — the aws-sdk-kotlin package, e.g.
 *       {@code aws.sdk.kotlin.services.sqs}.</li>
 * </ul>
 */
public final class SalvoClientCodegen implements SmithyBuildPlugin {
    @Override
    public String getName() {
        return "salvo-client-codegen";
    }

    @Override
    public void execute(PluginContext context) {
        Settings settings = Settings.from(context.getSettings());
        Generator generator = new Generator(context.getModel(), settings);
        for (Map.Entry<String, String> file : generator.generate().entrySet()) {
            context.getFileManifest().writeFile(file.getKey(), file.getValue());
        }
    }
}
