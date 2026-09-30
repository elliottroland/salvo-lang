package salvo.codegen;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

import software.amazon.smithy.aws.traits.ServiceTrait;
import software.amazon.smithy.kotlin.codegen.core.NamingKt;
import software.amazon.smithy.kotlin.codegen.utils.CaseUtilsKt;
import software.amazon.smithy.model.Model;
import software.amazon.smithy.model.knowledge.TopDownIndex;
import software.amazon.smithy.model.shapes.BlobShape;
import software.amazon.smithy.model.shapes.BooleanShape;
import software.amazon.smithy.model.shapes.ByteShape;
import software.amazon.smithy.model.shapes.DoubleShape;
import software.amazon.smithy.model.shapes.EnumShape;
import software.amazon.smithy.model.shapes.FloatShape;
import software.amazon.smithy.model.shapes.IntegerShape;
import software.amazon.smithy.model.shapes.ListShape;
import software.amazon.smithy.model.shapes.LongShape;
import software.amazon.smithy.model.shapes.MapShape;
import software.amazon.smithy.model.shapes.MemberShape;
import software.amazon.smithy.model.shapes.OperationShape;
import software.amazon.smithy.model.shapes.ServiceShape;
import software.amazon.smithy.model.shapes.Shape;
import software.amazon.smithy.model.shapes.ShapeId;
import software.amazon.smithy.model.shapes.ShortShape;
import software.amazon.smithy.model.shapes.StringShape;
import software.amazon.smithy.model.shapes.StructureShape;
import software.amazon.smithy.model.shapes.TimestampShape;
import software.amazon.smithy.model.traits.DefaultTrait;
import software.amazon.smithy.model.traits.DocumentationTrait;
import software.amazon.smithy.model.traits.HttpHeaderTrait;
import software.amazon.smithy.model.traits.RequiredTrait;
import software.amazon.smithy.model.traits.StreamingTrait;
import software.amazon.smithy.model.traits.TitleTrait;

/**
 * Walks one service's selected operations and renders four files:
 *
 * <ul>
 *   <li>{@code <module>.sv} — the Salvo surface: enums (unions of unit tags
 *       namespaced under the type, plus an open {@code Unknown} arm), structures,
 *       the service's error union, the effect, and a recording fake;</li>
 *   <li>{@code <module>/host.sv} — the {@code threadsafe platform handler} over
 *       the platform's SDK;</li>
 *   <li>{@code platform/<module>/host.rs} and {@code .kt} — the host glue, which
 *       converts between the Salvo values and the SDK's, runs each call on the
 *       SDK's own async machinery and completes the {@code Reply} from there.</li>
 * </ul>
 *
 * <p>A {@code @streaming} blob is a {@code stream.InStream}, which makes the
 * structure holding it a {@code linear struct} with a {@code close} beside it;
 * the glue registers a response body in the host's stream table and reads a
 * request body out of it [stream-table]. A timestamp is a {@code time.Instant}.
 *
 * <p>Anything the model uses that this generator does not map yet — documents,
 * event streams, unions, big numbers — stops generation with an error naming
 * the shape, rather than emitting something approximate.
 */
final class Generator {
    private final Model model;
    private final Settings settings;
    private final ServiceShape service;
    private final List<OperationShape> operations = new ArrayList<>();
    /** Structures in first-reached order, with the Salvo name each renders as. */
    private final Map<ShapeId, String> structs = new LinkedHashMap<>();
    /** Enums (as value types) in first-reached order. */
    private final Map<ShapeId, String> enums = new LinkedHashMap<>();
    /** Every modeled error of the selected operations, by Salvo name. */
    private final TreeSet<String> errorNames = new TreeSet<>();
    private final Map<String, StructureShape> errorShapes = new TreeMap<>();
    private final Set<ShapeId> inputs = new LinkedHashSet<>();
    private final Set<ShapeId> outputs = new LinkedHashSet<>();
    /** The `@streaming` member of each structure that has one (Smithy allows one, top-level). */
    private final Map<ShapeId, MemberShape> streaming = new LinkedHashMap<>();
    /** Whether any selected shape holds a timestamp. */
    private boolean usesTime = false;

    Generator(Model model, Settings settings) {
        this.model = model;
        this.settings = settings;
        this.service = model.expectShape(settings.service(), ServiceShape.class);
    }

    // ================================================================ walk ===

    Map<String, String> generate() {
        Set<OperationShape> all = TopDownIndex.of(model).getContainedOperations(service);
        for (String name : settings.operations()) {
            OperationShape op = all.stream()
                    .filter(o -> o.getId().getName().equals(name))
                    .findFirst()
                    .orElseThrow(() -> fail(name, "is not an operation of " + service.getId()));
            operations.add(op);
        }
        for (OperationShape op : operations) {
            input(op).ifPresent(s -> {
                inputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Input");
                walkStruct(s);
            });
            output(op).ifPresent(s -> {
                outputs.add(s.getId());
                structs.put(s.getId(), op.getId().getName() + "Output");
                walkStruct(s);
            });
            for (ShapeId e : op.getErrors(service)) {
                StructureShape es = model.expectShape(e, StructureShape.class);
                errorNames.add(e.getName());
                errorShapes.put(e.getName(), es);
                walkStruct(es);
            }
        }
        Map<String, String> files = new LinkedHashMap<>();
        String path = settings.module().replace('.', '/');
        files.put(path + ".sv", salvoSurface());
        files.put(path + "/host.sv", salvoHost());
        files.put("platform/" + path + "/host.sv.kt", kotlinTemplate());
        files.put("platform/" + path + "/host.sv.rs", rustTemplate());
        return files;
    }

    private Optional<StructureShape> input(OperationShape op) {
        return op.getInput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    private Optional<StructureShape> output(OperationShape op) {
        return op.getOutput().filter(id -> !id.toString().equals("smithy.api#Unit"))
                .map(id -> model.expectShape(id, StructureShape.class));
    }

    /** A structure's members in model order, without the ones the settings omit. */
    private List<MemberShape> members(StructureShape s) {
        List<MemberShape> out = new ArrayList<>();
        for (MemberShape m : s.getAllMembers().values()) {
            if (!settings.omitMembers().contains(m.getId().toString())) {
                out.add(m);
            } else if (m.hasTrait(RequiredTrait.class)) {
                throw fail(m.getId().toString(), "is required, so it cannot be omitted");
            }
        }
        return out;
    }

    private void walkStruct(StructureShape s) {
        for (MemberShape m : members(s)) {
            walkType(model.expectShape(m.getTarget()), m);
        }
    }

    private void walkType(Shape t, MemberShape via) {
        if (t.hasTrait(StreamingTrait.class)) {
            // Smithy puts a streaming member only at the top of an operation's
            // input or output (and one per structure); an event stream is a
            // streaming *union*, which is not mapped yet.
            ShapeId holder = via.getContainer();
            if (!(t instanceof BlobShape)) {
                throw fail(via.getId().toString(), "is an event stream, which this generator does not map yet");
            }
            if (!inputs.contains(holder) && !outputs.contains(holder)) {
                throw fail(via.getId().toString(), "is a streaming member outside an operation's input or output");
            }
            if (streaming.containsKey(holder) && !streaming.get(holder).equals(via)) {
                throw fail(holder.toString(), "has two streaming members");
            }
            streaming.put(holder, via);
            return;
        }
        if (t instanceof TimestampShape) {
            usesTime = true;
            return;
        }
        if (t instanceof EnumShape e) {
            enums.putIfAbsent(e.getId(), e.getId().getName());
        } else if (t instanceof StructureShape s) {
            if (!structs.containsKey(s.getId()) && !s.hasTrait("smithy.api#error")) {
                structs.put(s.getId(), s.getId().getName());
                walkStruct(s);
            }
        } else if (t instanceof ListShape l) {
            walkType(model.expectShape(l.getMember().getTarget()), l.getMember());
        } else if (t instanceof MapShape mp) {
            Shape key = model.expectShape(mp.getKey().getTarget());
            if (!(key instanceof StringShape)) {
                throw fail(mp.getId().toString(), "has a key that is not a string");
            }
            walkType(model.expectShape(mp.getValue().getTarget()), mp.getValue());
        } else if (!(t instanceof StringShape || t instanceof IntegerShape || t instanceof ShortShape
                || t instanceof ByteShape || t instanceof LongShape || t instanceof BooleanShape
                || t instanceof FloatShape || t instanceof DoubleShape || t instanceof BlobShape)) {
            throw fail(via.getId().toString(), "targets a " + t.getType() + ", which this generator does not map yet");
        }
    }

    private static IllegalStateException fail(String what, String why) {
        return new IllegalStateException("salvo-client-codegen: `" + what + "` " + why);
    }

    // =============================================================== names ===

    private static final Set<String> SALVO_KEYWORDS = Set.of(
            "fn", "let", "struct", "qualifier", "effect", "handler", "params", "type", "intrinsic",
            "platform", "import", "as", "of", "with", "canbe", "provenance", "refn", "rename", "is",
            "if", "elif", "else", "when", "while", "for", "in", "return", "break", "continue", "state",
            "use", "try", "true", "false", "proj", "once", "linear", "holds", "export", "copy");
    private static final Set<String> RUST_KEYWORDS = Set.of(
            "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do",
            "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let",
            "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
            "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
            "virtual", "where", "while", "yield");
    private static final Set<String> KOTLIN_KEYWORDS = Set.of(
            "as", "break", "class", "continue", "do", "else", "false", "for", "fun", "if", "in",
            "interface", "is", "null", "object", "package", "return", "super", "this", "throw", "true",
            "try", "typealias", "typeof", "val", "var", "when", "while");

    /** smithy-rs's snake case: the shared word-boundary splitter, lowered and joined. */
    static String snake(String name) {
        return String.join("_", CaseUtilsKt.splitOnWordBoundaries(name)).toLowerCase();
    }

    private static String pascal(String name) {
        return CaseUtilsKt.toPascalCase(name);
    }

    /** A member's Salvo field name: snake case, a Salvo keyword suffixed with `_`. */
    private static String salvoField(MemberShape m) {
        String s = snake(m.getMemberName());
        return SALVO_KEYWORDS.contains(s) ? s + "_" : s;
    }

    /** A Salvo name as the Rust emitter spells it (`rs_ident`). */
    private static String rsIdent(String salvo) {
        return RUST_KEYWORDS.contains(salvo) ? "r#" + salvo : salvo;
    }

    /** A Salvo name as the Kotlin emitter spells it (`kt_ident`). */
    private static String ktIdent(String salvo) {
        return KOTLIN_KEYWORDS.contains(salvo) ? "`" + salvo + "`" : salvo;
    }

    /** The SDK's Rust accessor for a member. */
    private static String rsAccessor(MemberShape m) {
        String s = snake(m.getMemberName());
        return RUST_KEYWORDS.contains(s) ? "r#" + s : s;
    }

    /** The SDK's Kotlin property for a member — smithy-kotlin's own rule. */
    private static String ktMember(MemberShape m) {
        return NamingKt.defaultName(m);
    }

    private String rsCrate() {
        return settings.rustCrate();
    }

    private String ktModel() {
        return settings.kotlinPackage() + ".model";
    }

    private String ktClient() {
        String sdkId = service.getTrait(ServiceTrait.class).map(ServiceTrait::getSdkId)
                .orElse(service.getId().getName());
        return settings.kotlinPackage() + "." + NamingKt.clientName(sdkId) + "Client";
    }

    /** A shape's type name in aws-sdk-rust: smithy-rs Pascal-cases it (`ObjectCannedAcl`). */
    private static String rsSdkName(String modelName) {
        return pascal(modelName);
    }

    /** A shape's type name in aws-sdk-kotlin, by smithy-kotlin's own rule. */
    private String ktSdkName(ShapeId id) {
        return NamingKt.defaultName(model.expectShape(id), service);
    }

    /** A structure's SDK type, Rust. */
    private String rsSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return snake(op.getId().getName()) + "::"
                        + rsSdkName(op.getId().getName()) + "Output";
            }
        }
        if (model.expectShape(id).hasTrait("smithy.api#error")) {
            return "sdk::error::" + rsSdkName(id.getName());
        }
        return "sdk::" + rsSdkName(id.getName());
    }

    /** A structure's SDK type, Kotlin (operation shapes are renamed Request/Response). */
    private String ktSdkStruct(ShapeId id) {
        for (OperationShape op : operations) {
            if (input(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktAlias(NamingKt.capitalizedDefaultName(op) + "Request");
            }
            if (output(op).map(Shape::getId).filter(id::equals).isPresent()) {
                return ktAlias(NamingKt.capitalizedDefaultName(op) + "Response");
            }
        }
        return ktAlias(ktSdkName(id));
    }

    /** Kotlin model types the glue names, each imported once as `Sdk<Name>`. */
    private final Set<String> ktAliases = new TreeSet<>();

    /**
     * An SDK model type under its import alias, `SdkMessage`: Kotlin cannot
     * alias a package, and a wildcard import would collide with the Salvo
     * struct of the same name.
     */
    private String ktAlias(String modelName) {
        ktAliases.add(modelName);
        return "Sdk" + modelName;
    }

    private String effectOp(OperationShape op) {
        return snake(op.getId().getName());
    }

    /** What an operation answers instead of its output: the service's error or an [AwsError]. */
    private String errorUnion() {
        return settings.effect() + "Failure";
    }

    /** The service's one error struct. */
    private String errorStruct() {
        return settings.effect() + "Error";
    }

    /** The literal union of the service's error codes. */
    private String errorCodes() {
        return settings.effect() + "ErrorCode";
    }

    /**
     * The members an error shape carries besides its message — S3's
     * `InvalidObjectState` has a storage class and an access tier — which become
     * optional fields of the one error struct, set for that code only.
     */
    private Map<String, MemberShape> extraErrorMembers() {
        Map<String, MemberShape> out = new LinkedHashMap<>();
        for (StructureShape es : errorShapes.values()) {
            for (MemberShape m : members(es)) {
                if (m.getMemberName().equalsIgnoreCase("message")) continue;
                out.putIfAbsent(salvoField(m), m);
            }
        }
        return out;
    }

    /**
     * Every code the service may send for a modeled error, mapped to the model's
     * name: the model name itself, and the legacy `@awsQueryError` code where
     * the protocol is `awsQueryCompatible` (SQS's `QueueDoesNotExist` arrives
     * as `AWS.SimpleQueueService.NonExistentQueue` on one SDK). Both backends
     * normalize through this, so a code reads the same on either.
     */
    private Map<String, String> wireCodes() {
        Map<String, String> out = new TreeMap<>();
        for (Map.Entry<String, StructureShape> e : errorShapes.entrySet()) {
            e.getValue().findTrait("aws.protocols#awsQueryError").ifPresent(t -> {
                String code = t.toNode().expectObjectNode().expectStringMember("code").getValue();
                if (!code.equals(e.getKey())) out.put(code, e.getKey());
            });
        }
        return out;
    }

    // =============================================================== types ===

    private boolean isStreaming(MemberShape m) {
        return target(m).hasTrait(StreamingTrait.class);
    }

    /** Whether a structure holds a stream, which makes it a `linear struct`. */
    private boolean isLinear(ShapeId struct) {
        return streaming.containsKey(struct);
    }

    /** A member's Salvo type, without optionality. */
    private String salvoType(Shape t) {
        if (t.hasTrait(StreamingTrait.class)) return "InStream";
        if (t instanceof TimestampShape) return "Instant";
        if (t instanceof EnumShape e) return enums.get(e.getId());
        if (t instanceof StructureShape s) {
            return s.hasTrait("smithy.api#error") ? s.getId().getName() : structs.get(s.getId());
        }
        if (t instanceof ListShape l) return "List<" + salvoType(model.expectShape(l.getMember().getTarget())) + ">";
        if (t instanceof MapShape m) return "Map<Str, " + salvoType(model.expectShape(m.getValue().getTarget())) + ">";
        if (t instanceof StringShape) return "Str";
        if (t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape) return "Int";
        if (t instanceof LongShape) return "Long";
        if (t instanceof BooleanShape) return "Bool";
        if (t instanceof FloatShape) return "Float";
        if (t instanceof DoubleShape) return "Double";
        if (t instanceof BlobShape) return "Bytes";
        throw fail(t.getId().toString(), "has no Salvo type");
    }

    /**
     * Whether a member is present on every value: required, or defaulted — a
     * streaming member always is, since its `@default` is the empty body.
     */
    private boolean present(MemberShape m) {
        return m.hasTrait(RequiredTrait.class) || isStreaming(m) || defaultOf(m) != null;
    }

    /** A literal Salvo default for a `@default` member, or null (never for a stream). */
    private String defaultOf(MemberShape m) {
        if (isStreaming(m)) return null;
        return m.getTrait(DefaultTrait.class).map(d -> {
            var n = d.toNode();
            if (n.isNullNode()) return null;
            if (n.isBooleanNode()) return String.valueOf(n.expectBooleanNode().getValue());
            if (n.isNumberNode()) return n.expectNumberNode().getValue().toString();
            if (n.isStringNode()) return "\"" + n.expectStringNode().getValue().replace("\"", "\\\"") + "\"";
            throw fail(m.getId().toString(), "has a non-scalar @default, which this generator does not map yet");
        }).orElse(null);
    }

    private Shape target(MemberShape m) {
        return model.expectShape(m.getTarget());
    }

    // ================================================================ docs ===

    /**
     * The first paragraph of a shape's documentation, as plain text. Admonitions
     * (`<important>`, `<note>`) are not the summary, so they are skipped — S3's
     * `PutObject` opens with an end-of-support notice.
     */
    private static String docText(Shape s) {
        String html = s.getTrait(DocumentationTrait.class).map(DocumentationTrait::getValue).orElse("");
        html = html.replaceAll("(?s)<(important|note)>.*?</\\1>", "").trim();
        int end = html.indexOf("</p>");
        if (end >= 0) html = html.substring(0, end);
        String text = html.replaceAll("</?(p|li|ul|ol|br|dd|dt|dl)[^>]*>", " ").replaceAll("<[^>]+>", "")
                .replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"")
                .replace("&#39;", "'").replace("&amp;", "&")
                .replaceAll("\\s+", " ").replaceAll(" ([.,;:)])", "$1").trim();
        return text;
    }

    /** `// `-comment lines at [indent], wrapped to 80 columns. */
    private static String comment(String text, String indent) {
        if (text.isEmpty()) return "";
        StringBuilder out = new StringBuilder();
        StringBuilder line = new StringBuilder();
        int width = 80 - indent.length() - 3;
        for (String word : text.split(" ")) {
            if (line.length() > 0 && line.length() + 1 + word.length() > width) {
                out.append(indent).append("// ").append(line).append('\n');
                line.setLength(0);
            }
            if (line.length() > 0) line.append(' ');
            line.append(word);
        }
        if (line.length() > 0) out.append(indent).append("// ").append(line).append('\n');
        return out.toString();
    }

    private String header(String what) {
        return "// Generated by `salvo-client-codegen` from `" + settings.modelFile() + "`; do not\n"
                + "// edit — regenerate with `./gradlew generate` in `modules/aws/codegen`.\n";
    }

    // ======================================================= Salvo surface ===

    private String salvoSurface() {
        StringBuilder out = new StringBuilder();
        String title = service.getTrait(TitleTrait.class).map(TitleTrait::getValue).orElse(settings.effect());
        out.append(comment(title + ", as a Salvo effect: every operation takes a `Reply` and returns at once, "
                + "answering when the service does [platform-reply].", ""));
        out.append("//\n");
        out.append(comment(docText(service), ""));
        out.append("//\n");
        out.append(comment("The host implementation is `" + settings.module() + ".host`'s `Host"
                + settings.effect() + "`; `Fake" + settings.effect() + "` below is a recording double "
                + "that needs no SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\n");
        if (!streaming.isEmpty()) out.append("import stream\n");
        if (usesTime) out.append("import time.Instant\n");

        for (Map.Entry<ShapeId, String> e : enums.entrySet()) {
            out.append('\n').append(salvoEnum(model.expectShape(e.getKey(), EnumShape.class)));
        }
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            out.append('\n').append(salvoStruct(model.expectShape(s.getKey(), StructureShape.class), s.getValue()));
        }
        out.append(salvoErrors());

        // The effect.
        out.append('\n');
        out.append(comment(title + ". Every member hands its [reply] to the provider and returns at once; "
                + "the answer is the operation's output or a `Checked` [" + errorUnion() + "].", ""));
        out.append("export effect ").append(settings.effect()).append(" {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append(comment(docText(op), "    "));
            out.append("    ").append(effectSignature(op)).append('\n');
        }
        out.append("}\n");

        // The recording fake.
        String calls = settings.effect() + "Calls";
        out.append('\n');
        out.append(comment("What a [Fake" + settings.effect() + "] was asked, in order: one operation name per call.", ""));
        out.append("export effect ").append(calls).append(" {\n");
        out.append("    // The names of the operations called so far (`").append(effectOp(operations.get(0)))
                .append("`, …).\n");
        out.append("    fn calls() -> List<Str>\n}\n");
        out.append('\n');
        out.append(comment("A recording double for [" + settings.effect() + "]: every call is noted by operation "
                + "name — read them through [" + calls + "] — and answered with an empty output where the "
                + "operation's output has no required members, otherwise with `AwsError { code: \"NotStubbed\" }`. "
                + (streaming.isEmpty() ? "" : "A body it is handed is closed unread, and a body it answers with "
                        + "is empty, minted in the `Streams` in scope. ")
                + "A test that needs real answers implements [" + settings.effect() + "] itself.", ""));
        // A fake that hands out or takes in bodies does so through the
        // `Streams` in scope, as every other producer does [stream-layer].
        String deps = streaming.isEmpty() ? "" : " [Streams]";
        out.append("export handler Fake").append(settings.effect()).append("()").append(deps).append(" of ")
                .append(settings.effect()).append(", ").append(calls).append(" {\n");
        out.append("    recorded: Mut List<Str> = mut_list_of()\n");
        for (OperationShape op : operations) {
            out.append('\n');
            out.append("    ").append(effectSignature(op).replace("\n    =>", " =>")).append(" {\n");
            out.append("        recorded.add(\"").append(effectOp(op)).append("\")\n");
            if (input(op).map(i -> isLinear(i.getId())).orElse(false)) {
                // The body is not read; closing it is what discharges the input.
                // A missing length is refused as the host refuses it, so a test
                // against the fake fails where production would.
                String len = salvoField(lengthOf(input(op).get()));
                out.append("        let unsized = input.").append(len).append(" is None\n");
                out.append("        let closed = close(input)\n");
                out.append("        if closed is Err {\n            ignore(closed)\n        }\n");
                out.append("        if unsized {\n            reply.send(").append(missingLength(op)).append(")\n")
                        .append("            return None\n        }\n");
            }
            out.append("        reply.send(").append(fakeAnswer(op)).append(")\n");
            out.append("    }\n");
        }
        out.append("\n    fn calls() -> List<Str> {\n        return copy(recorded)\n    }\n}\n");
        return out.toString();
    }

    private String effectSignature(OperationShape op) {
        StringBuilder sig = new StringBuilder("fn ").append(effectOp(op)).append('(');
        List<String> moved = new ArrayList<>();
        input(op).ifPresent(s -> {
            sig.append("input: ").append(structs.get(s.getId())).append(", ");
            moved.add("!input");
        });
        sig.append("reply: Reply<").append(replyPayload(op)).append(">) -> None\n    => ");
        moved.add("!reply");
        sig.append(String.join(", ", moved));
        return sig.toString();
    }

    private String replyPayload(OperationShape op) {
        String out = output(op).map(s -> structs.get(s.getId())).orElse("None");
        return "Ok " + out + " | Err Checked<" + errorUnion() + ">";
    }

    /** The answer to a streamed input without its length, in Salvo. */
    private String missingLength(OperationShape op) {
        return "err(checked<" + errorUnion() + ">(AwsError { code: \"MissingContentLength\", message: \""
                + missingLengthText(op) + "\" }))";
    }

    private String missingLengthText(OperationShape op) {
        return settings.effect() + " " + op.getId().getName() + " streams its body, so the input needs "
                + salvoField(lengthOf(input(op).get())) + ": the body's length in bytes";
    }

    private String fakeAnswer(OperationShape op) {
        Optional<StructureShape> out = output(op);
        if (out.isEmpty()) return "ok(None)";
        // A stream is present on every value, and the empty one is an empty body.
        boolean stubbable = members(out.get()).stream()
                .noneMatch(m -> present(m) && !isStreaming(m));
        if (stubbable) {
            List<String> fields = new ArrayList<>();
            for (MemberShape m : members(out.get())) {
                if (isStreaming(m)) fields.add(salvoField(m) + ": from_bytes(bytes_of())");
            }
            String body = fields.isEmpty() ? " {}" : " { " + String.join(", ", fields) + " }";
            return "ok(" + structs.get(out.get().getId()) + body + ")";
        }
        return "err(checked<" + errorUnion() + ">(AwsError { code: \"NotStubbed\", message: \"Fake"
                + settings.effect() + " cannot answer " + effectOp(op) + "\" }))";
    }

    /**
     * The service's errors as data: named code groups (`errorGroups` in the
     * settings), the code union — every modeled error of the selected
     * operations, and `Other Str` for the rest (throttling, access denied, a
     * code this model predates) — the one error struct, and the failure union
     * with [AwsError] for a call that got no response.
     */
    private String salvoErrors() {
        StringBuilder out = new StringBuilder("\n");
        List<String> arms = new ArrayList<>();
        Set<String> grouped = new LinkedHashSet<>();
        for (Map.Entry<String, String> g : settings.errorGroups().entrySet()) {
            List<String> codes = new ArrayList<>();
            for (String name : errorNames) {
                if (name.startsWith(g.getValue())) codes.add("\"" + name + "\"");
            }
            if (codes.isEmpty()) continue;
            grouped.addAll(codes);
            out.append(comment("The " + errorStruct() + " codes starting `" + g.getValue() + "`, as one group: `if "
                    + "e.code is " + g.getKey() + "` asks whether a failure was one of them.", ""));
            out.append("export type ").append(g.getKey()).append(" = ").append(String.join("\n    | ", codes))
                    .append("\n\n");
            arms.add(g.getKey());
        }
        for (String name : errorNames) {
            if (!grouped.contains("\"" + name + "\"")) arms.add("\"" + name + "\"");
        }
        arms.add("Other Str");
        out.append(comment("A code " + settings.effect() + " answers with: one arm per error the model names for "
                + "these operations, and `Other` for anything else the service sends — throttling, an "
                + "authorization failure, a code newer than this model. A legacy wire code is normalized to the "
                + "model's name, so a code reads the same on every backend.", ""));
        out.append("export type ").append(errorCodes()).append(" = ").append(String.join("\n    | ", arms))
                .append("\n\n");
        out.append(comment("The service answered with an error: what it said, and the HTTP status it said it "
                + "with.", ""));
        out.append("export struct ").append(errorStruct()).append(" {\n");
        out.append("    code: ").append(errorCodes()).append(",\n");
        out.append("    message: Str,\n");
        out.append("    // The HTTP status of the response.\n");
        out.append("    status: Int,\n");
        out.append("    // The service's id for the request, when it sent one: what AWS support asks for.\n");
        Map<String, MemberShape> extras = extraErrorMembers();
        out.append("    request_id: Str? = None").append(extras.isEmpty() ? "\n" : ",\n");
        int i = 0;
        for (Map.Entry<String, MemberShape> x : extras.entrySet()) {
            String owner = x.getValue().getContainer().getName();
            out.append(comment("Set for `" + owner + "` only. " + docText(x.getValue()), "    "));
            out.append("    ").append(x.getKey()).append(": ").append(salvoType(target(x.getValue())))
                    .append("? = None").append(++i < extras.size() ? ",\n" : "\n");
        }
        out.append("}\n\n");
        out.append(comment("Everything an operation of [" + settings.effect() + "] can answer with instead of its "
                + "output: the service's error, or an [AwsError] when there was no answer to read.", ""));
        out.append("export type ").append(errorUnion()).append(" = ").append(errorStruct()).append(" | AwsError\n");
        return out.toString();
    }

    private String salvoEnum(EnumShape e) {
        String name = enums.get(e.getId());
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(e).isEmpty() ? name + ", one literal per value the model names." : docText(e), ""));
        out.append("//\n");
        out.append(comment("Open, as Smithy enums are: a value this model does not name arrives as the `Other` arm "
                + "[type-literal]. At run time a plain string, the value as it is on the wire.", ""));
        List<String> arms = new ArrayList<>();
        for (String v : e.getEnumValues().values()) arms.add("\"" + v.replace("\"", "\\\"") + "\"");
        arms.add("Other Str");
        out.append("export type ").append(name).append(" = ").append(String.join("\n    | ", arms)).append('\n');
        return out.toString();
    }

    private String salvoStruct(StructureShape s, String name) {
        StringBuilder out = new StringBuilder();
        out.append(comment(docText(s), ""));
        List<MemberShape> members = new ArrayList<>(members(s));
        List<String> omitted = new ArrayList<>();
        for (MemberShape m : s.getAllMembers().values()) {
            if (!members.contains(m)) omitted.add("`" + salvoField(m) + "`");
        }
        if (!omitted.isEmpty()) {
            if (!docText(s).isEmpty()) out.append("//\n");
            out.append(comment("Not mapped: " + String.join(", ", omitted) + " — the SDKs customize "
                    + (omitted.size() == 1 ? "it" : "them") + " away from the model (`omitMembers` in "
                    + "smithy-build.json).", ""));
        }
        if (members.isEmpty()) {
            out.append("export struct ").append(name).append(" {}\n");
            return out.toString();
        }
        boolean linear = isLinear(s.getId());
        if (linear) {
            if (!docText(s).isEmpty()) out.append("//\n");
            out.append(comment("Linear, because it holds a stream: take the stream out (`let {"
                    + salvoField(streaming.get(s.getId())) + "} = value`) and close it when done, or give "
                    + "the whole value up with [close] [linear-group].", ""));
            if (inputs.contains(s.getId())) {
                String len = salvoField(lengthOf(s));
                out.append("//\n");
                out.append(comment("The " + salvoField(streaming.get(s.getId())) + " is streamed as it is read, "
                        + "so [" + len + "] must be its length in bytes — for a file, `metadata(path)`'s `size`. "
                        + "Without it the call answers `AwsError { code: \"MissingContentLength\" }` and sends "
                        + "nothing; a stream shorter or longer than it answers `AwsError { code: \"StreamFailed\" }`.",
                        ""));
            }
        }
        out.append(linear ? "export linear struct " : "export struct ").append(name).append(" {\n");
        for (int i = 0; i < members.size(); i++) {
            MemberShape m = members.get(i);
            out.append(comment(docText(m), "    "));
            String ty = salvoType(target(m));
            out.append("    ").append(salvoField(m)).append(": ");
            String def = defaultOf(m);
            if (m.hasTrait(RequiredTrait.class) || isStreaming(m)) {
                out.append(ty);
            } else if (def != null) {
                out.append(ty).append(" = ").append(def);
            } else {
                out.append(ty).append("? = None");
            }
            out.append(i + 1 < members.size() ? ",\n" : "\n");
        }
        out.append("}\n");
        if (linear) {
            String field = salvoField(streaming.get(s.getId()));
            out.append('\n');
            out.append(comment("Gives up [value] without reading its " + field + ": the stream is closed and "
                    + "the other members are dropped. The discharger of [" + name + "] [linear-group].", ""));
            out.append("export fn close(value: ").append(name)
                    .append(") [Streams] -> Ok None | Err Checked<StreamError> => !value {\n");
            out.append("    return close(value.").append(field).append(")\n}\n");
        }
        return out.toString();
    }


    // ============================================================ Rust glue ===


    /** A Salvo struct or enum name as the Rust emitter spells it (dot-names flattened). */
    private static String rsType(String salvoName) {
        return salvoName.replace(".", "");
    }








    /**
     * The member carrying a streaming input's length — the one bound to the
     * `Content-Length` header. A streamed request body must say its length
     * (user decision 2026-09-30: no silent buffering), so an input with a
     * stream and no such member is refused here rather than generated.
     */
    private MemberShape lengthOf(StructureShape s) {
        for (MemberShape m : members(s)) {
            if (m.getTrait(HttpHeaderTrait.class).map(h -> h.getValue().equalsIgnoreCase("Content-Length")).orElse(false)
                    && (target(m) instanceof LongShape || target(m) instanceof IntegerShape)) {
                return m;
            }
        }
        throw fail(s.getId().toString(), "streams a request body but has no Content-Length member to say its length");
    }

    /** The streaming member of an operation's input or output, or null. */
    private MemberShape streamOf(Optional<StructureShape> s) {
        return s.map(x -> streaming.get(x.getId())).orElse(null);
    }

    /** How a stream the glue registers describes itself: `S3 GetObject body`. */
    private String bodySource(OperationShape op) {
        return settings.effect() + " " + op.getId().getName() + " " + snake(streamOf(output(op)).getMemberName());
    }








    private String rustStreamHelpers() {
        String bs = rsCrate() + "::primitives::ByteStream";
        return """

                /// [stream-table] Registers a response body in the host's stream table,
                /// answering its handle. The table reads synchronously, from a Salvo worker
                /// or the stream host's reader thread — never from a task on the SDK's
                /// runtime — so each read blocks on the next chunk through [rt].
                fn salvo_register_body(source: &str, body: %1$s, rt: &tokio::runtime::Handle) -> i64 {
                    let reader = SalvoBody { rt: rt.clone(), body, chunk: Vec::new(), at: 0 };
                    crate::scheduler::salvo_stream_register_in(source.to_string(), Box::new(reader), 0)
                }

                /// A response body as `std::io::Read`.
                struct SalvoBody {
                    rt: tokio::runtime::Handle,
                    body: %1$s,
                    chunk: Vec<u8>,
                    at: usize,
                }

                impl std::io::Read for SalvoBody {
                    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                        while self.at == self.chunk.len() {
                            match self.rt.block_on(self.body.try_next()) {
                                Ok(Some(bytes)) => {
                                    self.chunk = bytes.to_vec();
                                    self.at = 0;
                                }
                                Ok(None) => return Ok(0),
                                Err(e) => return Err(std::io::Error::other(e)),
                            }
                        }
                        let n = buf.len().min(self.chunk.len() - self.at);
                        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
                        self.at += n;
                        Ok(n)
                    }
                }

                /// [stream-table] A request body streamed out of the host's table: a
                /// blocking thread reads it in chunks into a small channel the SDK's
                /// body drains, so memory stays at a few chunks whatever the size. The
                /// body is exactly `length` bytes — the SDKs sign and checksum a body of
                /// known length — and a stream that ends early or runs on past it is a
                /// failure, recorded in the answer's slot and sent to the SDK as an error
                /// frame. Not retryable: a stream is read once.
                fn salvo_upload(
                    body: std::sync::Arc<std::sync::Mutex<crate::scheduler::SalvoIn>>,
                    length: i64,
                ) -> (SalvoUpload, std::sync::Arc<std::sync::Mutex<Option<String>>>) {
                    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<bytes::Bytes>>(4);
                    let problem = std::sync::Arc::new(std::sync::Mutex::new(None));
                    let record = problem.clone();
                    tokio::task::spawn_blocking(move || {
                        let mut stream = body.lock().unwrap();
                        let fault = |source: &str, f: crate::scheduler::SalvoFault| match f {
                            crate::scheduler::SalvoFault::Utf8 => format!("{source}: not valid UTF-8"),
                            crate::scheduler::SalvoFault::Failed(m) => format!("{source}: {m}"),
                        };
                        let mut left = length as u64;
                        let failed: Option<String> = loop {
                            let mut buf = Vec::new();
                            if left == 0 {
                                break None;
                            }
                            match stream.read_up_to(&mut buf, left.min(65536) as usize) {
                                Ok(0) => {
                                    break Some(format!(
                                        "{}: ended after {} of the content_length of {length} bytes",
                                        stream.source,
                                        length as u64 - left
                                    ))
                                }
                                Ok(n) => {
                                    left -= n as u64;
                                    if left == 0 {
                                        // One byte past the end tells a stream longer than its
                                        // length — checked *before* the last chunk goes, so the
                                        // request fails rather than storing a truncated object.
                                        let mut extra = Vec::new();
                                        match stream.read_up_to(&mut extra, 1) {
                                            Ok(0) => {}
                                            Ok(_) => {
                                                break Some(format!(
                                                    "{}: longer than the content_length of {length} bytes",
                                                    stream.source
                                                ))
                                            }
                                            Err(f) => break Some(fault(&stream.source, f)),
                                        }
                                    }
                                    if tx.blocking_send(Ok(bytes::Bytes::from(buf))).is_err() {
                                        // The SDK gave up on the body; its error is the answer.
                                        break None;
                                    }
                                }
                                Err(f) => break Some(fault(&stream.source, f)),
                            }
                        };
                        if let Some(message) = failed {
                            *record.lock().unwrap() = Some(message.clone());
                            let _ = tx.blocking_send(Err(std::io::Error::other(message)));
                        }
                    });
                    (SalvoUpload { rx, length: length as u64 }, problem)
                }

                /// The SDK's side of [salvo_upload]: an `http_body::Body` of exactly
                /// `length` bytes over the channel.
                struct SalvoUpload {
                    rx: tokio::sync::mpsc::Receiver<std::io::Result<bytes::Bytes>>,
                    length: u64,
                }

                impl http_body::Body for SalvoUpload {
                    type Data = bytes::Bytes;
                    type Error = std::io::Error;

                    fn poll_frame(
                        mut self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Option<Result<http_body::Frame<bytes::Bytes>, std::io::Error>>> {
                        self.rx.poll_recv(cx).map(|next| next.map(|chunk| chunk.map(http_body::Frame::data)))
                    }

                    fn size_hint(&self) -> http_body::SizeHint {
                        http_body::SizeHint::with_exact(self.length)
                    }
                }
                """.formatted(bs);
    }


    // ========================================================== Kotlin glue ===














    private String kotlinStreamHelpers() {
        return """

                /**
                 * [stream-table] Registers a response body in the host's stream table,
                 * answering its handle. [closed] completes when the program closes the
                 * stream — or reads it to its end — which is what lets the SDK's response
                 * block return and release the connection.
                 */
                private fun salvoRegisterBody(
                    source: String,
                    body: aws.smithy.kotlin.runtime.content.ByteStream?,
                    closed: kotlinx.coroutines.CompletableDeferred<Unit>,
                ): Long {
                    val input = body?.toInputStream()
                        ?: java.io.ByteArrayInputStream(ByteArray(0))
                    return SalvoStreams.registerIn(source, SalvoBody(input, closed), 0)
                }

                private class SalvoBody(
                    input: java.io.InputStream,
                    private val closed: kotlinx.coroutines.CompletableDeferred<Unit>,
                ) : java.io.FilterInputStream(input) {
                    override fun close() {
                        try {
                            super.close()
                        } finally {
                            closed.complete(Unit)
                        }
                    }
                }

                /**
                 * [stream-table] A request body streamed out of the host's table as the SDK
                 * reads it: exactly [length] bytes, since the SDKs sign and checksum a body
                 * of known length. A stream that ends early, or runs on past [length], fails
                 * the read that finds it, which fails the request; the first failure is
                 * kept in [problem]. Not retryable: a stream is read once.
                 */
                private class SalvoUpload(private val stream: SalvoIn, private val length: Long) : java.io.InputStream() {
                    @Volatile var problem: String? = null
                    private var left = length

                    override fun read(): Int {
                        val one = ByteArray(1)
                        return if (read(one, 0, 1) <= 0) -1 else one[0].toInt() and 0xff
                    }

                    override fun read(b: ByteArray, off: Int, len: Int): Int {
                        if (len == 0) return 0
                        if (left == 0L) return -1
                        val got = next(minOf(len.toLong(), left).toInt())
                        if (got.isEmpty()) {
                            fail("${stream.source}: ended after ${length - left} of the content_length of $length bytes")
                        }
                        // One byte past the end tells a stream longer than its length —
                        // checked *before* the last chunk is handed over, since the SDK
                        // stops reading at the length and would store a truncated object.
                        if (left - got.size == 0L && next(1).isNotEmpty()) {
                            fail("${stream.source}: longer than the content_length of $length bytes")
                        }
                        System.arraycopy(got, 0, b, off, got.size)
                        left -= got.size
                        return got.size
                    }

                    /** The upload's first failure, if any. */
                    fun finish(): String? = problem

                    private fun next(max: Int): ByteArray =
                        try {
                            synchronized(stream) { stream.readUpTo(max) }
                        } catch (e: SalvoFaultException) {
                            val f = e.fault
                            fail("${stream.source}: " + if (f is SalvoFault.Failed) f.message else "not valid UTF-8")
                        }

                    private fun fail(message: String): Nothing {
                        if (problem == null) problem = message
                        throw java.io.IOException(message)
                    }
                }
                """;
    }

    // ======================================================= spliced host ===
    //
    // [host-splice] The host implementation is written *in Salvo*: one
    // `threadsafe platform handler` whose members and handler-level blocks are
    // fenced host code, every Salvo-side spelling (a field, a struct, a union
    // wrap, a type) a `@{…}` hole the compiler renders. What is left in the
    // text is the SDKs' own API and the documented host ABI [platform-abi].

    /** A hole reading [path] — a Salvo expression — as the compiler emits it. */
    private static String hole(String path) {
        return "@{" + path + "}";
    }

    /** A hole rendering the Salvo type [ty]. */
    private static String tyHole(String ty) {
        return "@{: " + ty + "}";
    }

    /** `@{name : ty}`: declares the host name [name] as a Salvo value of type [ty]. */
    private static String declHole(String name, String ty) {
        return "@{" + name + " : " + ty + "}";
    }

    /** A host name [name] seen as a Salvo value of type [ty], inside a hole. */
    private static String asSalvo(String name, String ty) {
        return "(`" + name + "` : " + ty + ")";
    }

    /**
     * `@{ value : place }` for an operation's answer: the value takes the type
     * of the host name [place] declared as the payload [host-splice], so the
     * payload union is written once, where [place] is declared.
     */
    private static String answer(String place, String value) {
        return "@{ " + value + " : " + place + " }";
    }

    /** The success answer, from the host name [value] declared as the output. */
    private String okAnswer(OperationShape op, String place, String value) {
        return output(op).isPresent() ? answer(place, "ok(" + value + ")") : answer(place, "ok(None)");
    }

    /** Declares the host name [name] as the operation's output: `let @{value : Out}`. */
    private String outputDecl(OperationShape op, String name) {
        return declHole(name, output(op).map(s -> structs.get(s.getId())).orElse("None"));
    }

    private String awsErrAnswer(String place, String code, String messageLeaf) {
        return answer(place, "err(checked<" + errorUnion() + ">(AwsError { code: \"" + code + "\", message: `"
                + messageLeaf + "` }))");
    }

    private String salvoHost() {
        StringBuilder out = new StringBuilder();
        String title = service.getTrait(TitleTrait.class).map(TitleTrait::getValue).orElse(settings.effect());
        out.append(comment("The host's " + title + ": [Host" + settings.effect() + "] implements `" + settings.module()
                + "`'s [" + settings.effect() + "] over aws-sdk-kotlin and aws-sdk-rust. Its code is the platform "
                + "templates beside this module, `platform/" + settings.module().replace('.', '/') + "/host.sv.kt` "
                + "and `.sv.rs` [host-splice].", ""));
        out.append("//\n");
        out.append(comment("Its own module, as `fs.host` is: a program that fakes the service never reaches "
                + "the host code, so it builds without the SDK.", ""));
        out.append("//\n");
        out.append(header(""));
        out.append("\nimport aws\nimport ").append(settings.module()).append('\n');
        if (!streaming.isEmpty()) out.append("import stream\n");
        if (usesTime) out.append("import time.Instant\n");
        out.append('\n');
        out.append(comment("[" + settings.effect() + "] through the platform's AWS SDK, configured by [config]: "
                + "each call runs on the SDK's own asynchronous machinery (coroutines on the JVM, tokio on "
                + "Rust) and completes its reply from there, so no Salvo worker waits. `threadsafe`: the SDK "
                + "client is safe to share, so the instance is used from every pool with no lock.", ""));
        out.append("export ").append(handlerDecl()).append('\n');
        return out.toString();
    }

    private String handlerDecl() {
        return "threadsafe platform handler Host" + settings.effect() + "(config: AwsConfig) of " + settings.effect();
    }

    /** A member's signature as a template marker writes it: no clause. */
    private String memberSig(OperationShape op) {
        String sig = effectSignature(op);
        return sig.substring(0, sig.indexOf("\n")).trim();
    }

    private String templateHeader(String lang) {
        return "// " + lang + " for `" + settings.module() + ".host`'s `Host" + settings.effect()
                + "` [host-splice]: host code, with Salvo\n"
                + "// between backticks, over the platform's AWS SDK.\n//\n"
                + header("").replace("regenerate with", "regenerate with");
    }

    private String kotlinTemplate() {
        StringBuilder members = new StringBuilder();
        for (OperationShape op : operations) {
            members.append('\n').append("    `").append(memberSig(op)).append("` {\n")
                    .append(indent(kotlinMember(op), 2)).append("    }\n");
        }
        String classBody = kotlinClassBody();
        StringBuilder out = new StringBuilder(templateHeader("Kotlin")).append('\n');
        for (String a : ktAliases) out.append("import ").append(ktModel()).append('.').append(a).append(" as Sdk").append(a).append('\n');
        out.append("import kotlinx.coroutines.launch\n");
        if (!streaming.isEmpty()) {
            out.append("import aws.smithy.kotlin.runtime.content.asByteStream\n");
            out.append("import aws.smithy.kotlin.runtime.content.toInputStream\n");
        }
        out.append("\n`").append(handlerDecl()).append("` {\n");
        out.append(indent(classBody.replace("import kotlinx.coroutines.launch\n", "")
                .replace("import aws.smithy.kotlin.runtime.content.asByteStream\n", "")
                .replace("import aws.smithy.kotlin.runtime.content.toInputStream\n", ""), 1));
        out.append(members).append("}\n");
        return toTemplate(out.toString());
    }

    private String rustTemplate() {
        StringBuilder out = new StringBuilder(templateHeader("Rust")).append('\n');
        out.append(rustModuleItems());
        String crate = rsCrate();
        out.append("\n// The handler's host fields: the tokio runtime each call runs on as a task,\n")
                .append("// and the SDK client, cheap to clone and safe to share.\n");
        out.append("`struct Host").append(settings.effect()).append("` {\n")
                .append("    rt: tokio::runtime::Runtime = salvo_runtime(),\n")
                .append("    client: ").append(crate).append("::Client = salvo_client(&rt, (&`config`).clone()),\n}\n");
        out.append("\n`").append(handlerDecl()).append("` {\n");
        boolean first = true;
        for (OperationShape op : operations) {
            if (!first) out.append('\n');
            first = false;
            out.append("    `").append(memberSig(op)).append("` {\n").append(indent(rustMember(op), 2)).append("    }\n");
        }
        out.append("}\n");
        return toTemplate(out.toString());
    }

    private static String indent(String body, int levels) {
        String pad = "    ".repeat(levels);
        StringBuilder out = new StringBuilder();
        for (String line : body.split("\n", -1)) out.append(line.isBlank() ? "" : pad + line).append('\n');
        while (out.toString().endsWith("\n\n")) out.setLength(out.length() - 1);
        return out.toString();
    }

    /**
     * Rewrites the generator's hole spelling into template markers: `@{: T}`
     * becomes `` `T` ``, `@{ e }` becomes `` `e` ``, and a `` `leaf` `` of host
     * code inside becomes `@{leaf}`.
     */
    static String toTemplate(String s) {
        StringBuilder out = new StringBuilder();
        int i = 0;
        while (i < s.length()) {
            if (s.startsWith("@{", i)) {
                int j = i + 2;
                int depth = 1;
                StringBuilder inner = new StringBuilder();
                while (j < s.length()) {
                    char c = s.charAt(j);
                    if (c == '`') {
                        int k = s.indexOf('`', j + 1);
                        inner.append("@{").append(s, j + 1, k).append('}');
                        j = k + 1;
                        continue;
                    }
                    if (c == '{') depth++;
                    if (c == '}' && --depth == 0) break;
                    inner.append(c);
                    j++;
                }
                String body = inner.toString().trim();
                if (body.startsWith(":")) body = body.substring(1).trim();
                out.append('`').append(body).append('`');
                i = j + 1;
            } else {
                out.append(s.charAt(i++));
            }
        }
        return out.toString();
    }

    /** A fenced block, its lines indented [indent] levels. */
    private static String fence(String lang, String body, int indent) {
        String pad = "    ".repeat(indent);
        StringBuilder out = new StringBuilder();
        out.append("    ```").append(lang).append('\n');
        for (String line : body.split("\n", -1)) {
            out.append(line.isBlank() ? "" : pad + line).append('\n');
        }
        while (out.toString().endsWith("\n\n")) out.setLength(out.length() - 1);
        out.append("    ```\n");
        return out.toString();
    }

    // ================================================================ Rust ===

    /** Salvo → SDK, from a *reference* expression [ref] to a Salvo value. */
    private String rsToSdk(Shape t, String ref) {
        if (t instanceof EnumShape e) {
            return "sdk::" + rsSdkName(e.getId().getName()) + "::from(" + ref + ".as_str())";
        }
        if (t instanceof StructureShape s) return "to_sdk_" + snake(structs.get(s.getId())) + "((" + ref + ").clone())";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsToSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape
                    ? "sdk::" + rsSdkName(key.getId().getName()) + "::from(k.as_str())"
                    : "k.clone()";
            return ref + ".iter().map(|(k, v)| (" + k + ", " + rsToSdk(target(m.getValue()), "v")
                    + ")).collect::<std::collections::HashMap<_, _>>()";
        }
        if (t instanceof StringShape) return ref + ".clone()";
        if (t instanceof BlobShape) return rsCrate() + "::primitives::Blob::new(" + ref + ".clone())";
        if (t instanceof TimestampShape) return "salvo_date_time((" + ref + ").clone())";
        return "*" + ref;
    }

    /** SDK → Salvo, from a reference expression [ref] to the SDK value. */
    private String rsFromSdk(Shape t, String ref) {
        if (t instanceof EnumShape) return ref + ".as_str().to_string()";
        if (t instanceof StructureShape s) return "from_sdk_" + snake(salvoType(s)) + "(" + ref + ")";
        if (t instanceof ListShape l) {
            return ref + ".iter().map(|e| " + rsFromSdk(target(l.getMember()), "e") + ").collect::<Vec<_>>()";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k.as_str().to_string()" : "k.clone()";
            // [rs-host-abi] A Salvo map is built from its entries, sorted so
            // iteration is the same on every run.
            return "{ let mut __es: Vec<_> = " + ref + ".iter().map(|(k, v)| (" + k + ", "
                    + rsFromSdk(target(m.getValue()), "v") + ")).collect(); "
                    + "__es.sort_by(|a, b| a.0.cmp(&b.0)); "
                    + "crate::collections::SalvoMap::from_entries::<crate::collections::HostHash, "
                    + "crate::collections::HostEq, _>(__es) }";
        }
        if (t instanceof StringShape) return ref + ".to_string()";
        if (t instanceof BlobShape) return ref + ".as_ref().to_vec()";
        if (t instanceof TimestampShape) return "salvo_instant(" + ref + ")";
        return "*" + ref;
    }

    private static boolean isPrimitive(Shape t) {
        return t instanceof IntegerShape || t instanceof ShortShape || t instanceof ByteShape
                || t instanceof LongShape || t instanceof BooleanShape || t instanceof FloatShape
                || t instanceof DoubleShape;
    }

    /** The Salvo value of one member, read from the SDK value [src] (a reference). */
    private String rsReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + rsAccessor(m) + "()";
        boolean present = present(m);
        if (t instanceof ListShape) {
            // smithy-rs answers `&[T]` for a list, empty when absent.
            if (present) return rsFromSdk(t, acc);
            return "{ let __s = " + acc + "; if __s.is_empty() { None } else { Some(" + rsFromSdk(t, "__s") + ") } }";
        }
        if (isPrimitive(t)) {
            return present ? rsFromSdk(t, "(&" + acc + ")") : acc + ".map(|x| " + rsFromSdk(t, "(&x)") + ")";
        }
        return present ? rsFromSdk(t, acc) : acc + ".map(|x| " + rsFromSdk(t, "x") + ")";
    }

    /** The `set_x(...)` argument for one member of the Salvo value at [salvo] (a Salvo expression). */
    private String rsSetArg(MemberShape m, String salvo) {
        Shape t = target(m);
        String field = hole(salvo + "." + salvoField(m));
        if (present(m)) return "Some(" + rsToSdk(t, "(&" + field + ")") + ")";
        return field + ".as_ref().map(|x| " + rsToSdk(t, "x") + ")";
    }

    /** Everything at module level beside the handler: its struct, `new`, and helpers. */
    private String rustModuleItems() {
        String crate = rsCrate();
        String effect = settings.effect();
        StringBuilder out = new StringBuilder();
        out.append("use ").append(crate).append("::error::ProvideErrorMetadata;\n");
        // The SDK's shapes under short names: `sdk::Message`, and each
        // operation's module (`send_message::SendMessageOutput`).
        out.append("use ").append(crate).append("::types as sdk;\n");
        List<String> ops = new ArrayList<>();
        for (OperationShape op : operations) ops.add(effectOp(op));
        out.append("use ").append(crate).append("::operation::{").append(String.join(", ", ops)).append("};\n\n");
        out.append("/// The tokio runtime a handler's calls run on.\n")
                .append("fn salvo_runtime() -> tokio::runtime::Runtime {\n")
                .append("    tokio::runtime::Builder::new_multi_thread()\n        .enable_all()\n        .build()\n")
                .append("        .expect(\"a tokio runtime for the AWS SDK\")\n}\n\n");
        out.append("/// The SDK client an `AwsConfig` describes.\n")
                .append("fn salvo_client(rt: &tokio::runtime::Runtime, ").append(declHole("config", "AwsConfig")).append(") -> ")
                .append(crate).append("::Client {\n")
                .append("    let shared = rt.block_on(salvo_aws_config(config.clone()));\n");
        if (settings.forcePathStyle()) {
            out.append("    ").append(crate).append("::Client::from_conf(\n")
                    .append("        ").append(crate).append("::config::Builder::from(&shared)\n")
                    .append("            .force_path_style(").append(hole("config.endpoint")).append(".is_some())\n")
                    .append("            .build(),\n    )\n");
        } else {
            out.append("    ").append(crate).append("::Client::new(&shared)\n");
        }
        out.append("}\n\n");
        String cfg = "cfg";
        String prof = asSalvo("p", "ProfileCredentials");
        out.append("""
                // The SDK configuration an `AwsConfig` describes: the region, the credentials
                // it names, and the endpoint override when there is one.
                #[allow(deprecated)]
                async fn salvo_aws_config(%s) -> aws_config::SdkConfig {
                    let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
                        .region(aws_config::Region::new(@{%s.region.code}));
                    if let Some(p) = @{profile_of(%s.credentials)} {
                        use aws_config::profile::profile_file::{ProfileFileKind, ProfileFiles};
                        let files = ProfileFiles::builder()
                            .with_file(ProfileFileKind::Credentials, salvo_expand_home(&@{%s.path}))
                            .build();
                        let provider = aws_config::profile::ProfileFileCredentialsProvider::builder()
                            .profile_files(files)
                            .profile_name(@{%s.profile})
                            .build();
                        loader = loader.credentials_provider(provider);
                    } else if @{uses_environment(%s.credentials)} {
                        loader = loader.credentials_provider(
                            aws_config::environment::EnvironmentVariableCredentialsProvider::new(),
                        );
                    }
                    if let Some(endpoint) = @{%s.endpoint} {
                        loader = loader.endpoint_url(endpoint);
                    }
                    loader.load().await
                }

                /// `~/` at the front of a path is the home directory, as a shell would read it.
                fn salvo_expand_home(path: &str) -> String {
                    match (path.strip_prefix("~/"), std::env::var("HOME")) {
                        (Some(rest), Ok(home)) => format!("{home}/{rest}"),
                        _ => path.to_string(),
                    }
                }
                """.formatted(declHole("cfg", "AwsConfig"), cfg, cfg, prof, prof, cfg, cfg));
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(rustFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey()) && !inputs.contains(s.getKey())) {
                out.append('\n').append(rustToSdk(shape, s.getValue()));
            }
        }
        out.append(rustFailure());
        if (usesTime) out.append(rustTimeHelpers());
        if (!streaming.isEmpty()) out.append(rustStreamHelpers());
        return out.toString();
    }

    private String rustMember(OperationShape op) {
        String crate = rsCrate();
        String name = effectOp(op);
        MemberShape inStream = streamOf(input(op));
        MemberShape outStream = streamOf(output(op));
        StringBuilder out = new StringBuilder();
        out.append("let reply = ").append(hole("reply")).append(".hosted();\n");
        out.append("let client = self.client.clone();\n");
        if (inStream != null) {
            // [stream-table] Taken out of the host's table now — the token was
            // given up with the input; a foreign handle traps [stream-provider].
            out.append("let body = crate::scheduler::salvo_stream_take_in(")
                    .append(hole("input." + salvoField(inStream) + ".handle")).append(");\n");
            out.append("let length = ").append(hole("input." + salvoField(lengthOf(input(op).get())))).append(";\n");
        }
        if (outStream != null) out.append("let rt = self.rt.handle().clone();\n");
        out.append("let call = client.").append(name).append("()");
        input(op).ifPresent(in -> {
            for (MemberShape m : members(in)) {
                if (isStreaming(m)) continue;
                out.append("\n    .set_").append(snake(m.getMemberName())).append('(').append(rsSetArg(m, "input")).append(')');
            }
        });
        out.append(";\n");
        out.append("self.rt.spawn(async move {\n");
        if (inStream != null) {
            // No length, no request: the body is released unread (user
            // decision 2026-09-30 — never buffer to find it out).
            out.append("    let length = match length {\n")
                    .append("        Some(n) if n >= 0 => n,\n")
                    .append("        _ => {\n            drop(body);\n")
                    .append("            let ").append(declHole("failed", replyPayload(op))).append(" = ")
                    .append(awsErrAnswer("failed", "MissingContentLength", "\"" + missingLengthText(op) + "\".to_string()"))
                    .append(";\n            reply.send(failed);\n            return;\n        }\n    };\n");
            out.append("    let (upload, problem) = salvo_upload(body, length);\n");
            out.append("    let call = call.set_").append(snake(inStream.getMemberName())).append("(Some(")
                    .append(crate).append("::primitives::ByteStream::from_body_1_x(upload)));\n");
        }
        String extra = rustExtra(op);
        out.append("    let ").append(declHole("answer", replyPayload(op))).append(" = match call.send().await {\n");
        String okValue = outStream != null
                ? "from_sdk_" + snake(structs.get(output(op).get().getId())) + "(out, &rt)"
                : output(op).map(s -> "from_sdk_" + snake(structs.get(s.getId())) + "(&out)").orElse("()");
        out.append("        Ok(out) => { let ").append(outputDecl(op, "value")).append(" = ").append(okValue).append("; ")
                .append(okAnswer(op, "answer", "value")).append(" }\n");
        out.append("        Err(e) => { let ").append(declHole("failure", "Checked<" + errorUnion() + ">"))
                .append(" = salvo_failure(e").append(extra.isEmpty() ? "" : ", " + extra)
                .append("); ").append(answer("answer", "err(failure)")).append(" }\n");
        out.append("    };\n");
        if (inStream != null) {
            // A body that failed, or did not match its length, is the answer
            // whatever the service said about the bytes it did get.
            out.append("    let problem = problem.lock().unwrap().take();\n")
                    .append("    let ").append(declHole("answer", replyPayload(op))).append(" = match problem {\n")
                    .append("        Some(message) => ").append(awsErrAnswer("answer", "StreamFailed", "message")).append(",\n")
                    .append("        None => answer,\n    };\n");
        }
        out.append("    reply.send(answer);\n});\n");
        return out.toString();
    }

    private String rustFromSdk(StructureShape s, String name) {
        MemberShape body = streaming.get(s.getId());
        StringBuilder out = new StringBuilder();
        if (body != null) {
            // The answer is taken by value: its body moves into the host's
            // stream table first — read later by whichever `Streams` the
            // program binds — and the other members are read after it.
            OperationShape op = operations.stream()
                    .filter(o -> output(o).map(Shape::getId).filter(s.getId()::equals).isPresent())
                    .findFirst().orElseThrow();
            out.append("fn from_sdk_").append(snake(name)).append("(mut v: ").append(rsSdkStruct(s.getId()))
                    .append(", rt: &tokio::runtime::Handle) -> ").append(tyHole(name)).append(" {\n");
            out.append("    let bytes = std::mem::replace(&mut v.").append(rsAccessor(body)).append(", ")
                    .append(rsCrate()).append("::primitives::ByteStream::from_static(b\"\"));\n");
            out.append("    let handle = salvo_register_body(\"").append(bodySource(op)).append("\", bytes, rt);\n");
            out.append("    let v = &v;\n");
        } else {
            out.append("fn from_sdk_").append(snake(name)).append("(v: &").append(rsSdkStruct(s.getId())).append(") -> ")
                    .append(tyHole(name)).append(" {\n");
        }
        out.append("    @{ ").append(name).append(" {");
        List<String> fields = new ArrayList<>();
        for (MemberShape m : members(s)) {
            if (m.equals(body)) {
                fields.add(salvoField(m) + ": InStream { handle: `handle` }");
            } else {
                fields.add(salvoField(m) + ": `" + rsReadField(m, "v") + "`");
            }
        }
        if (!fields.isEmpty()) out.append(' ').append(String.join(", ", fields)).append(' ');
        out.append("} }\n}\n");
        return out.toString();
    }

    private String rustToSdk(StructureShape s, String name) {
        String sdk = rsSdkStruct(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("fn to_sdk_").append(snake(name)).append("(").append(declHole("v", name)).append(") -> ").append(sdk)
                .append(" {\n    let b = ").append(sdk).append("::builder()");
        boolean fallible = false;
        for (MemberShape m : members(s)) {
            out.append("\n        .set_").append(snake(m.getMemberName())).append('(')
                    .append(rsSetArg(m, "v")).append(')');
            fallible |= m.hasTrait(RequiredTrait.class);
        }
        out.append(";\n    b.build()");
        if (fallible) out.append(".expect(\"every required member of ").append(name).append(" is set\")");
        out.append("\n}\n");
        return out.toString();
    }

    /**
     * The one error conversion, generic over every operation's error type: a
     * service error becomes the service's error struct — the code normalized
     * to the model's name, the message, the HTTP status, the request id — and
     * anything else (no response) an [AwsError]. The fields a rare error
     * carries (S3's `InvalidObjectState`) arrive through [extra].
     */
    private String rustFailure() {
        String crate = rsCrate();
        String es = errorStruct();
        Map<String, MemberShape> extras = extraErrorMembers();
        StringBuilder out = new StringBuilder();
        out.append("\n/// A code as the model names it: the shape id's name (`ns#Name`), and a legacy\n")
                .append("/// `@awsQueryError` code mapped to the model's.\n");
        out.append("fn salvo_code(code: &str) -> String {\n    let code = code.rsplit('#').next().unwrap_or(code);\n")
                .append("    match code {\n");
        for (Map.Entry<String, String> c : wireCodes().entrySet()) {
            out.append("        \"").append(c.getKey()).append("\" => \"").append(c.getValue()).append("\",\n");
        }
        out.append("        other => other,\n    }\n    .to_string()\n}\n\n");
        List<String> extraTys = new ArrayList<>();
        for (MemberShape m : extras.values()) extraTys.add(tyHole(salvoType(target(m)) + "?"));
        String extraTy = extraTys.size() == 1 ? extraTys.get(0) : "(" + String.join(", ", extraTys) + ")";
        out.append("fn salvo_failure<E>(\n    e: ").append(crate).append("::error::SdkError<E, ").append(crate)
                .append("::config::http::HttpResponse>,\n");
        if (!extras.isEmpty()) out.append("    extra: impl FnOnce(&E) -> ").append(extraTy).append(",\n");
        out.append(") -> ").append(tyHole("Checked<" + errorUnion() + ">")).append("\nwhere\n")
                .append("    E: ProvideErrorMetadata + std::error::Error + Send + Sync + 'static,\n{\n")
                .append("    use ").append(crate).append("::operation::RequestId;\n")
                .append("    let text = format!(\"{}\", ").append(crate).append("::error::DisplayErrorContext(&e));\n")
                .append("    let request_id = e.request_id().map(|r| r.to_string());\n")
                .append("    let kind = match &e {\n")
                .append("        ").append(crate).append("::error::SdkError::TimeoutError(_) => \"TimeoutError\",\n")
                .append("        ").append(crate).append("::error::SdkError::DispatchFailure(_) => \"DispatchFailure\",\n")
                .append("        ").append(crate).append("::error::SdkError::ResponseError(_) => \"ResponseError\",\n")
                .append("        _ => \"ConstructionFailure\",\n    };\n")
                .append("    let ").append(declHole("value", errorUnion())).append(" = match &e {\n")
                .append("        ").append(crate).append("::error::SdkError::ServiceError(ctx) => {\n")
                .append("            let err = ctx.err();\n")
                .append("            let code = salvo_code(err.code().unwrap_or(\"Unknown\"));\n")
                .append("            let message = err.message().unwrap_or(\"\").to_string();\n")
                .append("            let status = ctx.raw().status().as_u16() as i32;\n");
        List<String> fields = new ArrayList<>(List.of("code: `code`", "message: `message`", "status: `status`",
                "request_id: `request_id`"));
        if (!extras.isEmpty()) {
            out.append("            let extra = extra(err);\n");
            int i = 0;
            for (String f : extras.keySet()) {
                fields.add(f + ": `" + (extras.size() == 1 ? "extra" : "extra." + i) + "`");
                i++;
            }
        }
        out.append("            @{ ").append(es).append(" { ").append(String.join(", ", fields)).append(" } : ")
                .append(errorUnion()).append(" }\n        }\n");
        out.append("        _ => @{ AwsError { code: `kind.to_string()`, message: `text` } : ").append(errorUnion())
                .append(" },\n    };\n");
        out.append("    @{ checked<").append(errorUnion()).append(">(value) }\n}\n");
        return out.toString();
    }

    /** The `extra` closure for one operation: the fields its errors carry, or empty. */
    private String rustExtra(OperationShape op) {
        Map<String, MemberShape> extras = extraErrorMembers();
        if (extras.isEmpty()) return "";
        String crate = rsCrate();
        String errEnum = effectOp(op) + "::" + rsSdkName(op.getId().getName()) + "Error";
        List<String> none = new ArrayList<>();
        for (int i = 0; i < extras.size(); i++) none.add("None");
        String nothing = extras.size() == 1 ? "None" : "(" + String.join(", ", none) + ")";
        StringBuilder arms = new StringBuilder();
        for (ShapeId err : op.getErrors(service)) {
            StructureShape es = errorShapes.get(err.getName());
            List<String> reads = new ArrayList<>();
            boolean any = false;
            for (Map.Entry<String, MemberShape> x : extras.entrySet()) {
                MemberShape own = members(es).stream().filter(m -> salvoField(m).equals(x.getKey())).findFirst().orElse(null);
                if (own != null) {
                    any = true;
                    reads.add(present(own) ? "Some(" + rsReadField(own, "x") + ")" : rsReadField(own, "x"));
                } else {
                    reads.add("None");
                }
            }
            if (!any) continue;
            String value = reads.size() == 1 ? reads.get(0) : "(" + String.join(", ", reads) + ")";
            arms.append(errEnum).append("::").append(rsSdkName(err.getName())).append("(x) => ").append(value).append(", ");
        }
        if (arms.length() == 0) return "|_| " + nothing;
        return "|err| match err { " + arms + "_ => " + nothing + " }";
    }

    private String rustTimeHelpers() {
        String dt = rsCrate() + "::primitives::DateTime";
        return """

                /// A Smithy timestamp as a `time.Instant`: nanoseconds since the epoch,
                /// saturating outside `Instant`'s range (1678–2262).
                fn salvo_instant(t: &%1$s) -> @{: Instant} {
                    let nanos = t.as_nanos().clamp(i64::MIN as i128, i64::MAX as i128) as i64;
                    @{ Instant { nanos: `nanos` } }
                }

                /// A `time.Instant` as a Smithy timestamp.
                fn salvo_date_time(@{at : Instant}) -> %1$s {
                    %1$s::from_nanos(@{at.nanos} as i128).expect("an i64 of nanoseconds is a valid timestamp")
                }
                """.formatted(dt);
    }

    // ============================================================== Kotlin ===

    private static String stars(int n) {
        return "<" + String.join(", ", java.util.Collections.nCopies(n, "*")) + ">";
    }

    /** Salvo → SDK, from a non-null Kotlin expression [v]; [d] numbers nested lambda parameters. */
    private String ktToSdk(Shape t, String v, int d) {
        if (t instanceof EnumShape e) return ktAlias(ktSdkName(e.getId())) + ".fromValue(" + v + ")";
        if (t instanceof StructureShape s) return "toSdk" + structs.get(s.getId()) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktToSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? ktAlias(ktSdkName(key.getId())) + ".fromValue(k" + d + ")" : "k" + d;
            return v + ".entries.associate { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktToSdk(target(m.getValue()), "v" + d, d + 1) + " }";
        }
        if (t instanceof BlobShape) return v + ".toByteArray()";
        if (t instanceof TimestampShape) return "salvoDateTime(" + v + ")";
        return v;
    }

    /** SDK → Salvo, from a non-null expression [v]. */
    private String ktFromSdk(Shape t, String v, int d) {
        if (t instanceof EnumShape) return v + ".value";
        if (t instanceof StructureShape s) return "fromSdk" + salvoType(s) + "(" + v + ")";
        if (t instanceof ListShape l) {
            return v + ".map { e" + d + " -> " + ktFromSdk(target(l.getMember()), "e" + d, d + 1) + " }";
        }
        if (t instanceof MapShape m) {
            Shape key = target(m.getKey());
            String k = key instanceof EnumShape ? "k" + d + ".value" : "k" + d;
            String sortKey = key instanceof EnumShape ? "it.key.value" : "it.key";
            return v + ".entries.sortedBy { " + sortKey + " }.associate { (k" + d + ", v" + d + ") -> " + k + " to "
                    + ktFromSdk(target(m.getValue()), "v" + d, d + 1) + " }";
        }
        if (t instanceof BlobShape) return "salvo.SalvoBytes(" + v + ")";
        if (t instanceof TimestampShape) return "salvoInstant(" + v + ")";
        return v;
    }

    private static boolean ktNeedsConv(Shape t) {
        return !(t instanceof StringShape && !(t instanceof EnumShape)) && !isPrimitive(t);
    }

    private String ktReadField(MemberShape m, String src) {
        Shape t = target(m);
        String acc = src + "." + ktMember(m);
        if (present(m)) {
            return ktFromSdk(t, "(" + acc + " ?: error(\"missing required member " + m.getMemberName() + "\"))", 1);
        }
        return ktNeedsConv(t) ? acc + "?.let { " + ktFromSdk(t, "it", 1) + " }" : acc;
    }

    /** One member of the Salvo value at [salvo] (a Salvo expression), converted for the SDK. */
    private String ktSetField(MemberShape m, String salvo) {
        Shape t = target(m);
        String field = hole(salvo + "." + salvoField(m));
        if (present(m)) return ktToSdk(t, field, 1);
        return ktNeedsConv(t) ? field + "?.let { " + ktToSdk(t, "it", 1) + " }" : field;
    }

    /** The class body beside the members: the client, and every helper. */
    private String kotlinClassBody() {
        String effect = settings.effect();
        StringBuilder out = new StringBuilder();
        out.append("import kotlinx.coroutines.launch\n");
        if (!streaming.isEmpty()) {
            out.append("import aws.smithy.kotlin.runtime.content.asByteStream\n");
            out.append("import aws.smithy.kotlin.runtime.content.toInputStream\n");
        }
        out.append('\n');
        out.append("// The handler's host state: the SDK client, safe to share, and the scope\n")
                .append("// each call is a coroutine on.\n");
        out.append("private val scope = kotlinx.coroutines.CoroutineScope(\n")
                .append("    kotlinx.coroutines.SupervisorJob() + kotlinx.coroutines.Dispatchers.IO\n)\n");
        out.append("private val ").append(declHole("salvoConfig", "AwsConfig")).append(" = ").append(hole("config")).append('\n');
        String cfg = "salvoConfig";
        out.append("private val client = ").append(ktClient()).append(" {\n")
                .append("    region = @{").append(cfg).append(".region.code}\n")
                .append("    credentialsProvider = salvoCredentials(salvoConfig)\n")
                .append("    @{").append(cfg).append(".endpoint}?.let { endpointUrl = aws.smithy.kotlin.runtime.net.url.Url.parse(it) }\n");
        if (settings.forcePathStyle()) out.append("    forcePathStyle = @{").append(cfg).append(".endpoint} != null\n");
        out.append("}\n\ninit {\n    salvoCloseWhenMainEnds(client)\n}\n");
        for (Map.Entry<ShapeId, String> s : structs.entrySet()) {
            StructureShape shape = model.expectShape(s.getKey(), StructureShape.class);
            if (!inputs.contains(s.getKey())) out.append('\n').append(kotlinFromSdk(shape, s.getValue()));
            if (!outputs.contains(s.getKey())) out.append('\n').append(kotlinToSdk(shape, s.getValue()));
        }
        out.append(kotlinFailure());
        if (usesTime) out.append(kotlinTimeHelpers());
        if (!streaming.isEmpty()) out.append(kotlinStreamHelpers());
        out.append(kotlinCredentials());
        return out.toString();
    }

    private String kotlinMember(OperationShape op) {
        MemberShape inStream = streamOf(input(op));
        MemberShape outStream = streamOf(output(op));
        StringBuilder out = new StringBuilder();
        out.append("val host = ").append(hole("reply")).append(".hosted()\n");
        input(op).ifPresent(in -> out.append("val request = ").append(hole("input")).append('\n'));
        if (inStream != null) {
            // [stream-table] Taken out of the host's table now — the token was
            // given up with the input; a foreign handle traps [stream-provider].
            out.append("val body = SalvoStreams.takeIn(").append(hole("input." + salvoField(inStream) + ".handle")).append(")\n");
            out.append("val length = ").append(hole("input." + salvoField(lengthOf(input(op).get())))).append('\n');
        }
        out.append("scope.launch {\n");
        if (inStream != null) {
            out.append("    if (length == null || length < 0) {\n")
                    .append("        body.closeInput()\n")
                    .append("        val ").append(declHole("failed", replyPayload(op))).append(" = ")
                    .append(awsErrAnswer("failed", "MissingContentLength", "\"" + missingLengthText(op) + "\"")).append('\n')
                    .append("        host.send(failed)\n        return@launch\n    }\n");
            out.append("    val upload = SalvoUpload(body, length)\n");
        }
        String sdkRequest = input(op).map(in -> "toSdk" + structs.get(in.getId()) + "(request"
                + (inStream != null ? ", upload.asByteStream(length)" : "") + ")").orElse("");
        String call = "client." + NamingKt.defaultName(op) + "(" + sdkRequest + ")";
        if (outStream != null) {
            // The SDK's body lives only inside the response block, so the
            // block answers the reply itself and then waits for the program
            // to close the stream it was handed.
            out.append("    var sent = false\n");
            out.append("    val ").append(declHole("answer", "(" + replyPayload(op) + ")?")).append(" = try {\n");
            out.append("        ").append(call).append(" { response ->\n")
                    .append("            val closed = kotlinx.coroutines.CompletableDeferred<Unit>()\n")
                    .append("            val handle = salvoRegisterBody(\"").append(bodySource(op))
                    .append("\", response.").append(ktMember(outStream)).append(", closed)\n")
                    .append("            sent = true\n")
                    .append("            val ").append(outputDecl(op, "value")).append(" = fromSdk").append(structs.get(output(op).get().getId()))
                    .append("(response, handle)\n")
                    .append("            val ").append(declHole("success", replyPayload(op))).append(" = ").append(okAnswer(op, "success", "value")).append('\n')
                    .append("            host.send(success)\n")
                    .append("            closed.await()\n        }\n")
                    .append("        null\n");
        } else {
            out.append("    val ").append(declHole("answer", replyPayload(op))).append(" = try {\n");
            if (output(op).isPresent()) {
                out.append("        val ").append(outputDecl(op, "value")).append(" = fromSdk").append(structs.get(output(op).get().getId())).append('(')
                        .append(call).append(")\n");
                out.append("        ").append(okAnswer(op, "answer", "value")).append('\n');
            } else {
                out.append("        ").append(call).append('\n').append("        ").append(okAnswer(op, "answer", "")).append('\n');
            }
        }
        // With a streamed output `answer` is the optional payload (null once
        // the response block has sent it), so the failures name the payload
        // through a declared `failed` of their own.
        String caught = outStream != null ? "failed" : "answer";
        String failedDecl = outStream != null ? "val " + declHole("failed", replyPayload(op)) + " = " : "";
        String tail = outStream != null ? "\n        failed" : "";
        out.append("    } catch (e: aws.smithy.kotlin.runtime.ServiceException) {\n")
                .append("        val ").append(declHole("failure", errorStruct())).append(" = salvoFailure(e)\n")
                .append("        ").append(failedDecl).append(answer(caught, "err(checked<" + errorUnion() + ">(failure))"))
                .append(tail).append('\n');
        out.append("    } catch (e: Exception) {\n")
                .append("        val ").append(declHole("failure", "AwsError")).append(" = salvoAwsError(e)\n")
                .append("        ").append(failedDecl).append(answer(caught, "err(checked<" + errorUnion() + ">(failure))"))
                .append(tail).append("\n    }\n");
        if (outStream != null) {
            out.append("    if (!sent && answer != null) host.send(answer)\n}\n");
        } else if (inStream != null) {
            out.append("    val problem = upload.finish()\n")
                    .append("    body.closeInput()\n")
                    .append("    val ").append(declHole("result", replyPayload(op))).append(" = if (problem != null) ")
                    .append(awsErrAnswer("result", "StreamFailed", "problem")).append(" else answer\n")
                    .append("    host.send(result)\n}\n");
        } else {
            out.append("    host.send(answer)\n}\n");
        }
        return out.toString();
    }

    /** [rustFailure]'s counterpart: a service exception as the service's error struct. */
    private String kotlinFailure() {
        String es = errorStruct();
        StringBuilder out = new StringBuilder();
        out.append("\n/** A code as the model names it (see the Rust block's `salvo_code`). */\n");
        out.append("private fun salvoCode(code: String): String = when (val c = code.substringAfterLast('#')) {\n");
        for (Map.Entry<String, String> c : wireCodes().entrySet()) {
            out.append("    \"").append(c.getKey()).append("\" -> \"").append(c.getValue()).append("\"\n");
        }
        out.append("    else -> c\n}\n\n");
        out.append("private fun salvoFailure(e: aws.smithy.kotlin.runtime.ServiceException): ").append(tyHole(es)).append(" {\n")
                .append("    val meta = e.sdkErrorMetadata\n")
                .append("    val response = meta.protocolResponse as? aws.smithy.kotlin.runtime.http.response.HttpResponse\n")
                .append("    val code = salvoCode(meta.errorCode ?: \"Unknown\")\n")
                .append("    val message = meta.errorMessage ?: \"\"\n")
                .append("    val status = response?.status?.value ?: 0\n")
                .append("    val requestId = meta.requestId\n");
        List<String> fields = new ArrayList<>(List.of("code: `code`", "message: `message`", "status: `status`",
                "request_id: `requestId`"));
        int i = 0;
        for (Map.Entry<String, MemberShape> x : extraErrorMembers().entrySet()) {
            String owner = x.getValue().getContainer().getName();
            String local = "extra" + i++;
            out.append("    val ").append(local).append(" = (e as? ").append(ktAlias(ktSdkName(x.getValue().getContainer())))
                    .append(")?.let { ")
                    .append(ktReadField(x.getValue(), "it")).append(" }\n");
            fields.add(x.getKey() + ": `" + local + "`");
            if (owner.isEmpty()) break;
        }
        out.append("    return @{ ").append(es).append(" { ").append(String.join(", ", fields)).append(" } }\n}\n");
        out.append("""

                private fun salvoAwsError(e: Throwable): @{: AwsError} {
                    val code = (e as? aws.smithy.kotlin.runtime.ServiceException)?.sdkErrorMetadata?.errorCode
                        ?: e::class.simpleName ?: "Exception"
                    val message = e.message ?: e.toString()
                    return @{ AwsError { code: `code`, message: `message` } }
                }
                """);
        return out.toString();
    }

    private String kotlinFromSdk(StructureShape s, String name) {
        MemberShape body = streaming.get(s.getId());
        String sdk = ktSdkStruct(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("private fun fromSdk").append(name).append("(v: ").append(sdk)
                .append(body != null ? ", bodyHandle: Long" : "").append("): ").append(tyHole(name))
                .append(" = @{ ").append(name).append(" {");
        List<String> fields = new ArrayList<>();
        for (MemberShape m : members(s)) {
            fields.add(m.equals(body)
                    ? salvoField(m) + ": InStream { handle: `bodyHandle` }"
                    : salvoField(m) + ": `" + ktReadField(m, "v") + "`");
        }
        if (!fields.isEmpty()) out.append(' ').append(String.join(", ", fields)).append(' ');
        out.append("} }\n");
        return out.toString();
    }

    private String kotlinToSdk(StructureShape s, String name) {
        String sdk = ktSdkStruct(s.getId());
        MemberShape body = streaming.get(s.getId());
        StringBuilder out = new StringBuilder();
        out.append("private fun toSdk").append(name).append("(").append(declHole("v", name))
                .append(body != null ? ", bodyStream: aws.smithy.kotlin.runtime.content.ByteStream" : "").append("): ")
                .append(sdk).append(" = ").append(sdk).append(" {\n");
        for (MemberShape m : members(s)) {
            String value = m.equals(body) ? "bodyStream" : ktSetField(m, "v");
            out.append("    ").append(ktMember(m)).append(" = ").append(value).append('\n');
        }
        out.append("}\n");
        return out.toString();
    }

    private String kotlinTimeHelpers() {
        return """

                /** A Smithy timestamp as a `time.Instant`: nanoseconds since the epoch. */
                private fun salvoInstant(t: aws.smithy.kotlin.runtime.time.Instant): @{: Instant} {
                    val nanos = t.epochSeconds * 1_000_000_000L + t.nanosecondsOfSecond
                    return @{ Instant { nanos: `nanos` } }
                }

                /** A `time.Instant` as a Smithy timestamp. */
                private fun salvoDateTime(@{at : Instant}): aws.smithy.kotlin.runtime.time.Instant {
                    val nanos = @{at.nanos}
                    return aws.smithy.kotlin.runtime.time.Instant.fromEpochSeconds(
                        Math.floorDiv(nanos, 1_000_000_000L),
                        Math.floorMod(nanos, 1_000_000_000L).toInt(),
                    )
                }
                """;
    }

    private String kotlinCredentials() {
        String cfg = "cfg";
        String prof = asSalvo("profile", "ProfileCredentials");
        return """

                /** The SDK credentials provider an `AwsConfig`'s credentials name. */
                @OptIn(aws.sdk.kotlin.runtime.InternalSdkApi::class)
                private fun salvoCredentials(@{cfg : AwsConfig}): aws.smithy.kotlin.runtime.auth.awscredentials.CredentialsProvider {
                    val profile = @{profile_of(%1$s.credentials)}
                    if (profile != null) {
                        return aws.sdk.kotlin.runtime.auth.credentials.ProfileCredentialsProvider(
                            profileName = @{%2$s.profile},
                            configurationSource = aws.sdk.kotlin.runtime.config.profile.AwsConfigurationSource(
                                @{%2$s.profile},
                                salvoExpandHome("~/.aws/config"),
                                salvoExpandHome(@{%2$s.path}),
                            ),
                        )
                    }
                    if (@{uses_environment(%1$s.credentials)}) {
                        return aws.sdk.kotlin.runtime.auth.credentials.EnvironmentCredentialsProvider()
                    }
                    return aws.sdk.kotlin.runtime.auth.credentials.DefaultChainCredentialsProvider()
                }

                /** `~/` at the front of a path is the home directory, as a shell would read it. */
                private fun salvoExpandHome(path: String): String =
                    if (path.startsWith("~/")) (System.getenv("HOME") ?: System.getProperty("user.home")) + path.substring(1) else path

                /**
                 * Closes [client] once the program's `main` thread has ended. The SDK's
                 * HTTP engine (OkHttp) keeps a non-daemon dispatcher thread alive for 60s
                 * after its last call, which would hold the JVM open that long after the
                 * program is done; a Rust program exits when `main` returns, and this
                 * makes the JVM do the same. A stopgap: the runtime ending the process
                 * when `main` returns is on the roadmap, to be designed.
                 */
                private fun salvoCloseWhenMainEnds(client: AutoCloseable) {
                    val main = Thread.getAllStackTraces().keys.firstOrNull { it.name == "main" && it.threadGroup?.name == "main" }
                        ?: return
                    val closer = Thread {
                        main.join()
                        runCatching { client.close() }
                    }
                    closer.isDaemon = true
                    closer.name = "salvo-aws-close"
                    closer.start()
                }
                """.formatted(cfg, prof);
    }
}
