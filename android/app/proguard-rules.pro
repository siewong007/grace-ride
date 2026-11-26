# kotlinx.serialization
-keepattributes *Annotation*, InnerClasses, EnclosingMethod, Signature, Exceptions
-dontnote kotlinx.serialization.**
-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** { kotlinx.serialization.KSerializer serializer(...); }
-keep,includedescriptorclasses class org.pjstatecm.graceride.**$$serializer { *; }
-keepclassmembers class org.pjstatecm.graceride.** { *** Companion; }
-keepclasseswithmembers class org.pjstatecm.graceride.** { kotlinx.serialization.KSerializer serializer(...); }

# Preserve Navigation Compose @Serializable AppRoute names so KClass.qualifiedName matches SerialDescriptor.serialName
-keep class org.pjstatecm.graceride.ui.AppRoute { *; }
-keep class org.pjstatecm.graceride.ui.AppRoute$* { *; }

# Keep all @Serializable models, data objects (INSTANCE), fields, and synthetic constructors
-keepnames @kotlinx.serialization.Serializable class org.pjstatecm.graceride.**
-keepclassmembers @kotlinx.serialization.Serializable class org.pjstatecm.graceride.** {
    static *** Companion;
    static *** INSTANCE;
    <fields>;
    kotlinx.serialization.KSerializer serializer(...);
    synthetic <init>(...);
}
-keepclassmembers class org.pjstatecm.graceride.** implements kotlinx.serialization.KSerializer {
    static *** INSTANCE;
}

# Retrofit / OkHttp
-dontwarn okhttp3.**
-dontwarn okio.**
-dontwarn retrofit2.**
-keepclassmembers,allowshrinking,allowobfuscation interface * { @retrofit2.http.* <methods>; }
-dontwarn org.codehaus.mojo.animal_sniffer.IgnoreJRERequirement
-dontwarn javax.annotation.**

# DTOs are serialized reflectively via generated serializers; keep their fields.
-keepclassmembers class org.pjstatecm.graceride.data.api.** { <fields>; }
