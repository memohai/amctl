package com.example.amctl.di

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.preferencesDataStore
import com.example.amctl.data.repository.SettingsRepository
import com.example.amctl.data.repository.SettingsRepositoryImpl
import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityServiceProviderImpl
import com.example.amctl.services.accessibility.ActionExecutor
import com.example.amctl.services.accessibility.ActionExecutorImpl
import com.example.amctl.services.screencapture.ScreenCaptureProvider
import com.example.amctl.services.screencapture.ScreenCaptureProviderImpl
import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import javax.inject.Qualifier
import javax.inject.Singleton

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class IoDispatcher

private val Context.settingsDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "settings",
)

@Module
@InstallIn(SingletonComponent::class)
object AppModule {
    @Provides
    @Singleton
    fun provideDataStore(
        @ApplicationContext context: Context,
    ): DataStore<Preferences> = context.settingsDataStore

    @Provides
    @IoDispatcher
    fun provideIoDispatcher(): CoroutineDispatcher = Dispatchers.IO
}

@Module
@InstallIn(SingletonComponent::class)
abstract class RepositoryModule {
    @Binds
    @Singleton
    abstract fun bindSettingsRepository(impl: SettingsRepositoryImpl): SettingsRepository
}

@Module
@InstallIn(SingletonComponent::class)
abstract class ServiceModule {
    @Binds
    @Singleton
    abstract fun bindActionExecutor(impl: ActionExecutorImpl): ActionExecutor

    @Binds
    @Singleton
    abstract fun bindAccessibilityServiceProvider(impl: AccessibilityServiceProviderImpl): AccessibilityServiceProvider

    @Binds
    @Singleton
    abstract fun bindScreenCaptureProvider(impl: ScreenCaptureProviderImpl): ScreenCaptureProvider
}
